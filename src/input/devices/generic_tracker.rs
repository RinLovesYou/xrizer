use openvr as vr;
use openxr as xr;

use crate::{input::Profiles, runtime_extensions::mndx_xdev_space::Xdev};

use super::tracked_device::{
    BaseDevice, TrackedDevice, TrackedDeviceType, RESERVED_DEVICE_INDECES,
};

pub const MAX_GENERIC_TRACKERS: u32 = vr::k_unMaxTrackedDeviceCount - RESERVED_DEVICE_INDECES;

pub struct XrGenericTracker {
    base: BaseDevice,
    space: xr::Space,
    _name: String,
    _serial: String,
}

impl XrGenericTracker {
    pub fn new(index: vr::TrackedDeviceIndex_t, dev: Xdev) -> Self {
        assert!(
            index >= RESERVED_DEVICE_INDECES,
            "Generic Tracker created with a reserved device index {}",
            index
        );
        assert!(
            dev.space.is_some(),
            "Generic Tracker initialized without a space!"
        );

        let tracker = Self {
            base: BaseDevice::new(index, TrackedDeviceType::GenericTracker),
            space: dev.space.unwrap(),
            _name: dev.properties.name(),
            _serial: dev.properties.serial(),
        };

        tracker.set_interaction_profile(
            Profiles::get()
                .profile_from_name("/interaction_profiles/htc/vive_tracker_htcx")
                .unwrap(),
        );

        tracker
            .base
            .connected
            .store(true, std::sync::atomic::Ordering::Relaxed);

        tracker
    }
}

impl TrackedDevice for XrGenericTracker {
    fn get_pose(
        &self,
        xr_data: &crate::openxr_data::OpenXrData<impl crate::openxr_data::Compositor>,
        session_data: &crate::openxr_data::SessionData,
        origin: vr::ETrackingUniverseOrigin,
    ) -> Option<vr::TrackedDevicePose_t> {
        let (location, velocity) = self
            .space
            .relate(
                session_data.get_space_for_origin(origin),
                xr_data.display_time.get(),
            )
            .unwrap();

        Some(vr::space_relation_to_openvr_pose(location, velocity))
    }

    fn get_base_device(&self) -> &BaseDevice {
        &self.base
    }
}
