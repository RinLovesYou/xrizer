use crate::{
    clientcore::{Injected, Injector},
    input::{
        devices::tracked_device::{TrackedDevice, TrackedDeviceType},
        Input,
    },
    openxr_data::{RealOpenXrData, SessionData},
    set_property_error, tracy_span,
};
use glam::{Mat3, Quat, Vec3};
use log::{debug, trace, warn};
use openvr as vr;
use openxr as xr;
use std::ffi::{CStr, CString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct ConnectedHands {
    left: AtomicBool,
    right: AtomicBool,
}

#[derive(Copy, Clone)]
pub struct ViewData {
    pub flags: xr::ViewStateFlags,
    pub views: [xr::View; 2],
}

#[derive(Default)]
struct ViewCache {
    view: Option<ViewData>,
    local: Option<ViewData>,
    stage: Option<ViewData>,
}

impl ViewCache {
    fn get_views(
        &mut self,
        session: &SessionData,
        display_time: xr::Time,
        ty: xr::ReferenceSpaceType,
    ) -> ViewData {
        let data = match ty {
            xr::ReferenceSpaceType::VIEW => &mut self.view,
            xr::ReferenceSpaceType::LOCAL => &mut self.local,
            xr::ReferenceSpaceType::STAGE => &mut self.stage,
            other => panic!("unexpected reference space type: {other:?}"),
        };

        *data.get_or_insert_with(|| {
            let (flags, views) = session
                .session
                .locate_views(
                    xr::ViewConfigurationType::PRIMARY_STEREO,
                    display_time,
                    session.get_space_from_type(ty),
                )
                .expect("Couldn't locate views");

            ViewData {
                flags,
                views: views
                    .try_into()
                    .unwrap_or_else(|v: Vec<xr::View>| panic!("Expected 2 views, got {}", v.len())),
            }
        })
    }
}

#[derive(macros::InterfaceImpl)]
#[interface = "IVRSystem"]
#[versions(022, 021, 020, 019, 016, 015)]
pub struct System {
    openxr: Arc<RealOpenXrData>, // We don't need to test session restarting.
    input: Injected<Input<crate::compositor::Compositor>>,
    vtables: Vtables,
    views: Mutex<ViewCache>,
}

pub mod log_tags {
    pub const TRACKED_PROP: &str = "tracked_property";
}

impl System {
    pub fn new(openxr: Arc<RealOpenXrData>, injector: &Injector) -> Self {
        Self {
            openxr,
            input: injector.inject(),
            vtables: Default::default(),
            views: Mutex::default(),
        }
    }

    pub fn reset_views(&self) {
        std::mem::take(&mut *self.views.lock().unwrap());
    }

    pub fn get_views(&self, ty: xr::ReferenceSpaceType) -> ViewData {
        tracy_span!();
        let session = self.openxr.session_data.get();
        self.views
            .lock()
            .unwrap()
            .get_views(&session, self.openxr.display_time.get(), ty)
    }
}

impl vr::IVRSystem022_Interface for System {
    fn GetRecommendedRenderTargetSize(&self, width: *mut u32, height: *mut u32) {
        let views = self
            .openxr
            .instance
            .enumerate_view_configuration_views(
                self.openxr.system_id,
                xr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .unwrap();

        if !width.is_null() {
            unsafe { *width = views[0].recommended_image_rect_width };
        }

        if !height.is_null() {
            unsafe { *height = views[0].recommended_image_rect_height };
        }
    }
    fn GetProjectionMatrix(&self, eye: vr::EVREye, near_z: f32, far_z: f32) -> vr::HmdMatrix44_t {
        // https://github.com/ValveSoftware/openvr/wiki/IVRSystem::GetProjectionRaw
        let [mut left, mut right, mut up, mut down] = [0.0; 4];
        self.GetProjectionRaw(eye, &mut left, &mut right, &mut down, &mut up);

        let idx = 1.0 / (right - left);
        let idy = 1.0 / (up - down);
        let idz = 1.0 / (far_z - near_z);
        let sx = right + left;
        let sy = up + down;

        vr::HmdMatrix44_t {
            m: [
                [2.0 * idx, 0.0, sx * idx, 0.0],
                [0.0, 2.0 * idy, sy * idy, 0.0],
                [0.0, 0.0, -far_z * idz, -far_z * near_z * idz],
                [0.0, 0.0, -1.0, 0.0],
            ],
        }
    }
    fn GetProjectionRaw(
        &self,
        eye: vr::EVREye,
        left: *mut f32,
        right: *mut f32,
        top: *mut f32,
        bottom: *mut f32,
    ) {
        let ty = self
            .openxr
            .session_data
            .get()
            .current_origin_as_reference_space();
        let view = self.get_views(ty).views[eye as usize];

        // Top and bottom are flipped, for some reason
        unsafe {
            *left = view.fov.angle_left.tan();
            *right = view.fov.angle_right.tan();
            *bottom = view.fov.angle_up.tan();
            *top = view.fov.angle_down.tan();
        }
    }
    fn ComputeDistortion(
        &self,
        _: vr::EVREye,
        _: f32,
        _: f32,
        _: *mut vr::DistortionCoordinates_t,
    ) -> bool {
        crate::warn_unimplemented!("ComputeDistortion");
        false
    }
    fn GetEyeToHeadTransform(&self, eye: vr::EVREye) -> vr::HmdMatrix34_t {
        let views = self.get_views(xr::ReferenceSpaceType::VIEW).views;
        let view = views[eye as usize];
        let view_rot = view.pose.orientation;

        {
            tracy_span!("conversion");
            let rot = Mat3::from_quat(Quat::from_xyzw(
                view_rot.x, view_rot.y, view_rot.z, view_rot.w,
            ))
            .transpose();

            let gen_array = |translation, rot_axis: Vec3| {
                std::array::from_fn(|i| if i == 3 { translation } else { rot_axis[i] })
            };
            vr::HmdMatrix34_t {
                m: [
                    gen_array(view.pose.position.x, rot.x_axis),
                    gen_array(view.pose.position.y, rot.y_axis),
                    gen_array(view.pose.position.z, rot.z_axis),
                ],
            }
        }
    }
    fn GetTimeSinceLastVsync(&self, _: *mut f32, _: *mut u64) -> bool {
        todo!()
    }
    fn GetRuntimeVersion(&self) -> *const std::os::raw::c_char {
        static VERSION: &CStr = c"2.5.1";
        VERSION.as_ptr()
    }
    fn GetAppContainerFilePaths(&self, _: *mut std::os::raw::c_char, _: u32) -> u32 {
        todo!()
    }
    fn AcknowledgeQuit_Exiting(&self) {
        todo!()
    }
    fn PerformFirmwareUpdate(&self, _: vr::TrackedDeviceIndex_t) -> vr::EVRFirmwareError {
        todo!()
    }
    fn ShouldApplicationReduceRenderingWork(&self) -> bool {
        false
    }
    fn ShouldApplicationPause(&self) -> bool {
        false
    }
    fn IsSteamVRDrawingControllers(&self) -> bool {
        todo!()
    }
    fn IsInputAvailable(&self) -> bool {
        true
    }
    fn GetControllerAxisTypeNameFromEnum(
        &self,
        _: vr::EVRControllerAxisType,
    ) -> *const std::os::raw::c_char {
        todo!()
    }
    fn GetButtonIdNameFromEnum(&self, _: vr::EVRButtonId) -> *const std::os::raw::c_char {
        todo!()
    }
    fn TriggerHapticPulse(&self, _: vr::TrackedDeviceIndex_t, _: u32, _: std::os::raw::c_ushort) {
        crate::warn_unimplemented!("TriggerHapticPulse");
    }
    fn GetControllerStateWithPose(
        &self,
        origin: vr::ETrackingUniverseOrigin,
        device_index: vr::TrackedDeviceIndex_t,
        state: *mut vr::VRControllerState_t,
        state_size: u32,
        pose: *mut vr::TrackedDevicePose_t,
    ) -> bool {
        if self.GetControllerState(device_index, state, state_size) {
            unsafe {
                *pose.as_mut().unwrap() = self
                    .input
                    .get()
                    .unwrap()
                    .get_device_pose(device_index as usize, Some(origin))
                    .unwrap_or_default();
            }
            true
        } else {
            false
        }
    }
    fn GetControllerState(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
        state: *mut vr::VRControllerState_t,
        state_size: u32,
    ) -> bool {
        self.input
            .force(|_| Input::new(self.openxr.clone()))
            .get_legacy_controller_state(device_index, state, state_size)
    }
    fn GetHiddenAreaMesh(
        &self,
        eye: vr::EVREye,
        ty: vr::EHiddenAreaMeshType,
    ) -> vr::HiddenAreaMesh_t {
        if !self.openxr.enabled_extensions.khr_visibility_mask {
            return Default::default();
        }

        debug!("GetHiddenAreaMesh: area mesh type: {ty:?}");
        let mask_ty = match ty {
            vr::EHiddenAreaMeshType::Standard => xr::VisibilityMaskTypeKHR::HIDDEN_TRIANGLE_MESH,
            vr::EHiddenAreaMeshType::Inverse => xr::VisibilityMaskTypeKHR::VISIBLE_TRIANGLE_MESH,
            vr::EHiddenAreaMeshType::LineLoop => xr::VisibilityMaskTypeKHR::LINE_LOOP,
            vr::EHiddenAreaMeshType::Max => {
                warn!("Unexpectedly got EHiddenAreaMeshType::Max - returning default area mesh");
                return Default::default();
            }
        };

        let session_data = self.openxr.session_data.get();
        let mask = session_data
            .session
            .get_visibility_mask_khr(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                eye as u32,
                mask_ty,
            )
            .unwrap();

        trace!("openxr mask: {:#?} {:#?}", mask.indices, mask.vertices);

        let [mut left, mut right, mut top, mut bottom] = [0.0; 4];
        self.GetProjectionRaw(eye, &mut left, &mut right, &mut top, &mut bottom);

        // convert from indices + vertices to just vertices
        let vertices: Vec<_> = mask
            .indices
            .into_iter()
            .map(|i| {
                let v = mask.vertices[i as usize];

                // It is unclear to me why this scaling is necessary, but OpenComposite does it and
                // it seems to get games to use the mask correctly.
                let x_scaled = (v.x - left) / (right - left);
                let y_scaled = (v.y - top) / (bottom - top);
                vr::HmdVector2_t {
                    v: [x_scaled, y_scaled],
                }
            })
            .collect();

        trace!("vertices: {vertices:#?}");
        let count = vertices.len() / 3;
        // XXX: what are we supposed to do here? pVertexData is a random pointer and there's no
        // clear way for the application to deallocate it
        // fortunately it seems like applications don't call this often, so this leakage isn't a
        // huge deal.
        let vertices = Vec::leak(vertices).as_ptr();

        vr::HiddenAreaMesh_t {
            pVertexData: vertices,
            unTriangleCount: count as u32,
        }
    }
    fn GetEventTypeNameFromEnum(&self, _: vr::EVREventType) -> *const std::os::raw::c_char {
        todo!()
    }
    fn PollNextEventWithPose(
        &self,
        _: vr::ETrackingUniverseOrigin,
        _: *mut vr::VREvent_t,
        _: u32,
        _: *mut vr::TrackedDevicePose_t,
    ) -> bool {
        false
    }
    fn PollNextEvent(&self, event: *mut vr::VREvent_t, _size: u32) -> bool {
        use std::ptr::addr_of_mut as ptr;
        let devices = self.openxr.devices.read().unwrap();

        let device_state_event = |current_state, last_state: &AtomicBool, tracked_device| {
            last_state.store(current_state, Ordering::Relaxed);

            debug!(
                "sending tracked device {}connected",
                if current_state { "" } else { "not " }
            );

            // Since the VREvent_t struct can be a variable size, it seems a little dangerous to
            // create a reference to it, so we'll just operate through pointers.
            // The eventType, trackedDeviceIndex, and eventAgeSeconds fields have always existed.
            unsafe {
                ptr!((*event).eventType).write(if current_state {
                    vr::EVREventType::TrackedDeviceActivated as u32
                } else {
                    vr::EVREventType::TrackedDeviceDeactivated as u32
                });

                ptr!((*event).trackedDeviceIndex).write(tracked_device);
                ptr!((*event).eventAgeSeconds).write(0.0);
            }
        };

        for device in devices.get_devices().iter() {
            if device.connected() != device.last_connected_state().load(Ordering::Relaxed) {
                device_state_event(
                    device.connected(),
                    device.last_connected_state(),
                    device.device_index() as u32,
                );
                return true;
            }
        }

        false
    }

    fn GetPropErrorNameFromEnum(
        &self,
        _: vr::ETrackedPropertyError,
    ) -> *const std::os::raw::c_char {
        c"Unknown error".as_ptr()
    }
    fn GetStringTrackedDeviceProperty(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
        prop: vr::ETrackedDeviceProperty,
        value: *mut std::os::raw::c_char,
        size: u32,
        err: *mut vr::ETrackedPropertyError,
    ) -> u32 {
        debug!(target: log_tags::TRACKED_PROP, "requesting string property: {prop:?} ({device_index})");
        let devices = self.openxr.devices.read().unwrap();
        let device = devices.get_device(device_index as usize);

        if device.is_none() {
            set_property_error!(err, vr::ETrackedPropertyError::InvalidDevice);
            return 0;
        }

        let device = device.unwrap();

        if !device.connected() {
            set_property_error!(err, vr::ETrackedPropertyError::InvalidDevice);
            return 0;
        }

        let ret = device.get_string_property(prop, err);

        if ret.is_empty() {
            return 0;
        }

        let data = CString::new(ret).unwrap();

        let buf = if !value.is_null() && size > 0 {
            unsafe { std::slice::from_raw_parts_mut(value, size as usize) }
        } else {
            &mut []
        };

        let data =
            unsafe { std::slice::from_raw_parts(data.as_ptr(), data.to_bytes_with_nul().len()) };
        if buf.len() < data.len() {
            set_property_error!(err, vr::ETrackedPropertyError::BufferTooSmall);
        } else {
            buf[0..data.len()].copy_from_slice(data);
        }

        data.len() as u32
    }
    fn GetArrayTrackedDeviceProperty(
        &self,
        _: vr::TrackedDeviceIndex_t,
        _: vr::ETrackedDeviceProperty,
        _: vr::PropertyTypeTag_t,
        _: *mut std::os::raw::c_void,
        _: u32,
        _: *mut vr::ETrackedPropertyError,
    ) -> u32 {
        todo!()
    }
    fn GetMatrix34TrackedDeviceProperty(
        &self,
        _: vr::TrackedDeviceIndex_t,
        _: vr::ETrackedDeviceProperty,
        _: *mut vr::ETrackedPropertyError,
    ) -> vr::HmdMatrix34_t {
        todo!()
    }
    fn GetUint64TrackedDeviceProperty(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
        prop: vr::ETrackedDeviceProperty,
        err: *mut vr::ETrackedPropertyError,
    ) -> u64 {
        debug!(target: log_tags::TRACKED_PROP, "requesting uint64 property: {prop:?} ({device_index})");
        if !self.IsTrackedDeviceConnected(device_index) {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = vr::ETrackedPropertyError::InvalidDevice;
            }
        }
        if let Some(err) = unsafe { err.as_mut() } {
            *err = vr::ETrackedPropertyError::UnknownProperty;
        }

        0
    }
    fn GetInt32TrackedDeviceProperty(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
        prop: vr::ETrackedDeviceProperty,
        err: *mut vr::ETrackedPropertyError,
    ) -> i32 {
        debug!(target: log_tags::TRACKED_PROP, "requesting int32 property: {prop:?} ({device_index})");
        let devices = self.openxr.devices.read().unwrap();
        let device = devices.get_device(device_index as usize);

        if let Some(device) = device {
            if !device.connected() {
                set_property_error!(err, vr::ETrackedPropertyError::InvalidDevice);
                return 0;
            }

            return device.get_int32_property(prop, err);
        }

        0
    }
    fn GetFloatTrackedDeviceProperty(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
        prop: vr::ETrackedDeviceProperty,
        err: *mut vr::ETrackedPropertyError,
    ) -> f32 {
        debug!(target: log_tags::TRACKED_PROP, "requesting float property: {prop:?} ({device_index})");
        let devices = self.openxr.devices.read().unwrap();
        let device = devices.get_device(device_index as usize);
        if let Some(device) = device {
            if !device.connected() {
                set_property_error!(err, vr::ETrackedPropertyError::InvalidDevice);
                return 0.0;
            }

            return device.get_float_property(prop, err, self);
        }

        0.0
    }

    fn GetBoolTrackedDeviceProperty(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
        prop: vr::ETrackedDeviceProperty,
        err: *mut vr::ETrackedPropertyError,
    ) -> bool {
        debug!(target: log_tags::TRACKED_PROP, "requesting bool property: {prop:?} ({device_index})");
        let devices = self.openxr.devices.read().unwrap();
        let device = devices.get_device(device_index as usize);
        if let Some(device) = device {
            if !device.connected() {
                set_property_error!(err, vr::ETrackedPropertyError::InvalidDevice);
                return false;
            }

            return device.get_bool_property(prop, err);
        }

        false
    }

    fn IsTrackedDeviceConnected(&self, device_index: vr::TrackedDeviceIndex_t) -> bool {
        let devices = self.openxr.devices.read().unwrap();
        if let Some(dev) = devices.get_device(device_index as usize) {
            dev.connected()
        } else {
            false
        }
    }

    fn GetTrackedDeviceClass(&self, index: vr::TrackedDeviceIndex_t) -> vr::ETrackedDeviceClass {
        if !self.IsTrackedDeviceConnected(index) {
            vr::ETrackedDeviceClass::Invalid
        } else {
            TrackedDeviceType::from(index).into()
        }
    }
    fn GetControllerRoleForTrackedDeviceIndex(
        &self,
        index: vr::TrackedDeviceIndex_t,
    ) -> vr::ETrackedControllerRole {
        match index {
            x if TrackedDeviceType::try_from(x).is_ok() => {
                match TrackedDeviceType::try_from(x).unwrap() {
                    TrackedDeviceType::LeftHand => vr::ETrackedControllerRole::LeftHand,
                    TrackedDeviceType::RightHand => vr::ETrackedControllerRole::RightHand,
                    _ => vr::ETrackedControllerRole::Invalid,
                }
            }
            _ => vr::ETrackedControllerRole::Invalid,
        }
    }
    fn GetTrackedDeviceIndexForControllerRole(
        &self,
        role: vr::ETrackedControllerRole,
    ) -> vr::TrackedDeviceIndex_t {
        let devices = self.openxr.devices.read().unwrap();
        match role {
            vr::ETrackedControllerRole::LeftHand => {
                if devices
                    .get_controller(TrackedDeviceType::LeftHand)
                    .unwrap()
                    .connected()
                {
                    TrackedDeviceType::LeftHand as u32
                } else {
                    vr::k_unTrackedDeviceIndexInvalid
                }
            }
            vr::ETrackedControllerRole::RightHand => {
                if devices
                    .get_controller(TrackedDeviceType::RightHand)
                    .unwrap()
                    .connected()
                {
                    TrackedDeviceType::RightHand as u32
                } else {
                    vr::k_unTrackedDeviceIndexInvalid
                }
            }
            _ => vr::k_unTrackedDeviceIndexInvalid,
        }
    }
    fn ApplyTransform(
        &self,
        _: *mut vr::TrackedDevicePose_t,
        _: *const vr::TrackedDevicePose_t,
        _: *const vr::HmdMatrix34_t,
    ) {
        todo!()
    }
    fn GetTrackedDeviceActivityLevel(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
    ) -> vr::EDeviceActivityLevel {
        match device_index {
            vr::k_unTrackedDeviceIndex_Hmd => vr::EDeviceActivityLevel::UserInteraction,
            x if TrackedDeviceType::try_from(x).is_ok() => {
                if self.IsTrackedDeviceConnected(x) {
                    vr::EDeviceActivityLevel::UserInteraction
                } else {
                    vr::EDeviceActivityLevel::Unknown
                }
            }
            _ => vr::EDeviceActivityLevel::Unknown,
        }
    }
    fn GetSortedTrackedDeviceIndicesOfClass(
        &self,
        _: vr::ETrackedDeviceClass,
        _: *mut vr::TrackedDeviceIndex_t,
        _: u32,
        _: vr::TrackedDeviceIndex_t,
    ) -> u32 {
        0
    }
    fn GetRawZeroPoseToStandingAbsoluteTrackingPose(&self) -> vr::HmdMatrix34_t {
        todo!()
    }
    fn GetSeatedZeroPoseToStandingAbsoluteTrackingPose(&self) -> vr::HmdMatrix34_t {
        todo!()
    }
    fn GetDeviceToAbsoluteTrackingPose(
        &self,
        origin: vr::ETrackingUniverseOrigin,
        _seconds_to_photon_from_now: f32,
        pose_array: *mut vr::TrackedDevicePose_t,
        pose_count: u32,
    ) {
        self.input
            .force(|_| Input::new(self.openxr.clone()))
            .get_poses(
                unsafe { std::slice::from_raw_parts_mut(pose_array, pose_count as usize) },
                Some(origin),
            );
    }
    fn SetDisplayVisibility(&self, _: bool) -> bool {
        todo!()
    }
    fn IsDisplayOnDesktop(&self) -> bool {
        todo!()
    }
    fn GetOutputDevice(
        &self,
        device: *mut u64,
        texture_type: vr::ETextureType,
        instance: *mut vr::VkInstance_T,
    ) {
        if texture_type != vr::ETextureType::Vulkan {
            // Proton doesn't seem to properly translate this function, but it doesn't appear to
            // actually matter.
            log::error!("Unsupported texture type: {texture_type:?}");
            return;
        }

        unsafe {
            *device = self
                .openxr
                .instance
                .vulkan_graphics_device(self.openxr.system_id, instance as _)
                .expect("Failed to get vulkan physical device") as _;
        }
    }
    fn GetDXGIOutputInfo(&self, _: *mut i32) {
        todo!()
    }
    fn GetD3D9AdapterIndex(&self) -> i32 {
        todo!()
    }
}

impl vr::IVRSystem021On022 for System {
    fn ResetSeatedZeroPose(&self) {
        crate::warn_unimplemented!("ResetSeatedZeroPose");
    }
}

impl vr::IVRSystem020On021 for System {
    fn AcknowledgeQuit_UserPrompt(&self) {}
}

impl vr::IVRSystem019On020 for System {
    fn DriverDebugRequest(
        &self,
        _un_device_index: vr::TrackedDeviceIndex_t,
        _pch_request: *const std::os::raw::c_char,
        _pch_response_buffer: *mut std::os::raw::c_char,
        _un_response_buffer_size: u32,
    ) -> u32 {
        unimplemented!()
    }
}

impl vr::IVRSystem016On019 for System {
    fn IsInputFocusCapturedByAnotherProcess(&self) -> bool {
        false
    }
    fn ReleaseInputFocus(&self) {}
    fn CaptureInputFocus(&self) -> bool {
        true
    }
    fn GetOutputDevice(&self, _device: *mut u64, _texture_type: vr::ETextureType) {
        // TODO: figure out what to pass for the instance...
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clientcore::Injector;
    use std::ffi::CStr;
    use vr::IVRSystem022_Interface;

    #[test]
    fn unity_required_properties() {
        let xr = Arc::new(RealOpenXrData::new(&Injector::default()).unwrap());
        let injector = Injector::default();
        let system = System::new(xr, &injector);

        let test_prop = |property| {
            let mut err = vr::ETrackedPropertyError::Success;
            let len = system.GetStringTrackedDeviceProperty(
                vr::k_unTrackedDeviceIndex_Hmd,
                property,
                std::ptr::null_mut(),
                0,
                &mut err,
            );
            assert_eq!(err, vr::ETrackedPropertyError::BufferTooSmall);
            assert!(len > 0);
            let mut buf = vec![0; len as usize];

            let len = system.GetStringTrackedDeviceProperty(
                vr::k_unTrackedDeviceIndex_Hmd,
                property,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut err,
            );
            assert_eq!(err, vr::ETrackedPropertyError::Success);
            assert_eq!(len, buf.len() as u32);

            let slice = unsafe { std::slice::from_raw_parts(buf.as_ptr() as _, buf.len()) };
            CStr::from_bytes_with_nul(slice)
                .expect("Failed to convert returned buffer for {property:?} to CStr");
        };

        test_prop(vr::ETrackedDeviceProperty::SerialNumber_String);
        test_prop(vr::ETrackedDeviceProperty::ManufacturerName_String);
        test_prop(vr::ETrackedDeviceProperty::ControllerType_String);
    }
}
