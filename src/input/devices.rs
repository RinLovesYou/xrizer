use controller::XrController;
use enum_dispatch::enum_dispatch;
use generic_tracker::XrGenericTracker;
use hmd::XrHMD;
use tracked_device::{BaseDevice, TrackedDevice, TrackedDeviceType, RESERVED_DEVICE_INDECES};

use openvr as vr;
use openxr as xr;

use crate::{
    input::InteractionProfile,
    openxr_data::{OpenXrData, SessionData},
    runtime_extensions::mndx_xdev_space::Xdev,
};

pub mod controller;
pub mod generic_tracker;
pub mod hmd;
pub mod tracked_device;

// enum dispatch generates the trait implementation for this enum,
// so we can access the methods of the variants without having to match anything manually.
#[enum_dispatch(TrackedDevice)]
pub enum TrackedDeviceContainer {
    HMD(XrHMD),
    Controller(XrController),
    GenericTracker(XrGenericTracker),
}

pub struct TrackedDeviceList {
    pub devices: Vec<TrackedDeviceContainer>,
}

impl Default for TrackedDeviceList {
    fn default() -> Self {
        Self {
            devices: vec![XrHMD::new().into()],
        }
    }
}

#[allow(dead_code)]
impl TrackedDeviceList {
    pub fn new(xr_instance: &xr::Instance) -> Self {
        Self {
            devices: vec![
                XrHMD::new().into(),
                XrController::new(xr_instance, TrackedDeviceType::LeftHand).into(),
                XrController::new(xr_instance, TrackedDeviceType::RightHand).into(),
            ],
        }
    }

    pub fn push(&mut self, device: TrackedDeviceContainer) {
        self.devices.push(device);
    }

    pub fn get_device(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
    ) -> Option<&TrackedDeviceContainer> {
        self.devices.get(device_index as usize)
    }

    pub fn get_device_mut(
        &mut self,
        device_index: vr::TrackedDeviceIndex_t,
    ) -> Option<&mut TrackedDeviceContainer> {
        self.devices.get_mut(device_index as usize)
    }

    pub unsafe fn get_device_unchecked(
        &self,
        device_index: vr::TrackedDeviceIndex_t,
    ) -> &TrackedDeviceContainer {
        self.devices.get_unchecked(device_index as usize)
    }

    pub unsafe fn get_device_mut_unchecked(
        &mut self,
        device_index: vr::TrackedDeviceIndex_t,
    ) -> &mut TrackedDeviceContainer {
        self.devices.get_unchecked_mut(device_index as usize)
    }

    /// This function is only intended to be used for the HMD and controllers. For other devices, it'll return the first match.
    pub fn get_device_by_type(
        &self,
        device_type: TrackedDeviceType,
    ) -> Option<&TrackedDeviceContainer> {
        self.devices
            .iter()
            .find(|device| device.get_type() == device_type)
    }

    /// This function is only intended to be used for the HMD and controllers. For other devices, it'll return the first match.
    pub fn get_device_by_type_mut(
        &mut self,
        device_type: TrackedDeviceType,
    ) -> Option<&mut TrackedDeviceContainer> {
        self.devices
            .iter_mut()
            .find(|device| device.get_type() == device_type)
    }

    pub fn get_hmd(&self) -> &XrHMD {
        let hmd = unsafe { self.get_device_unchecked(0) };

        match hmd {
            TrackedDeviceContainer::HMD(hmd) => hmd,
            _ => panic!("HMD is not the first device in the list"),
        }
    }

    pub fn get_controller(&self, hand: TrackedDeviceType) -> &XrController {
        let controller = match hand {
            TrackedDeviceType::LeftHand => unsafe { self.get_device_unchecked(1) },
            TrackedDeviceType::RightHand => unsafe { self.get_device_unchecked(2) },
            _ => panic!("Invalid hand type"),
        };

        match controller {
            TrackedDeviceContainer::Controller(controller) => controller,
            _ => panic!("Controller is not the second or third device in the list"),
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, TrackedDeviceContainer> {
        self.devices.iter()
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }

    pub fn truncate(&mut self, len: usize) {
        self.devices.truncate(len);
    }

    pub fn create_generic_trackers(
        &mut self,
        xr_data: &OpenXrData<impl crate::openxr_data::Compositor>,
    ) -> xr::Result<()> {
        if xr_data.xdev_extension.is_none() {
            return Ok(());
        }

        let xdev_extension = xr_data.xdev_extension.as_ref().unwrap();

        log::info!("Creating generic trackers");

        let session = xr_data.session_data.get();

        let xdevs: Vec<Xdev> = xdev_extension
            .enumerate_xdevs(&session.session)?
            .into_iter()
            .filter(|device| {
                device.space.is_some()
                    && device.properties.name().to_lowercase().contains("tracker")
            })
            .collect();

        log::info!("Found {} generic trackers", xdevs.len());

        self.truncate(RESERVED_DEVICE_INDECES as usize);

        xdevs.into_iter().for_each(|xdev| {
            let tracker = XrGenericTracker::new(self.len() as u32, xdev);
            self.push(tracker.into());
        });

        Ok(())
    }
}
