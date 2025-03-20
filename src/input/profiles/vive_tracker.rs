use glam::Mat4;

use super::{
    InteractionProfile, MainAxisType, PathTranslation, ProfileProperties, Property,
    SkeletalInputBindings, StringToPath,
};
use crate::input::legacy::LegacyBindings;
use crate::openxr_data::Hand;

pub struct ViveTracker;

impl InteractionProfile for ViveTracker {
    fn properties(&self) -> &'static ProfileProperties {
        &ProfileProperties {
            model: c"Vive Tracker Handheld Object",
            openvr_controller_type: c"vive_tracker_handheld_object",
            render_model_name: Property::BothHands(c"vive_tracker"),
            main_axis: MainAxisType::Thumbstick,
        }
    }
    fn profile_path(&self) -> &'static str {
        "/interaction_profiles/htc/vive_tracker_htcx"
    }
    fn translate_map(&self) -> &'static [PathTranslation] {
        &[]
    }

    fn legacy_bindings(&self, _stp: &dyn StringToPath) -> LegacyBindings {
        LegacyBindings {
            grip_pose: vec![],
            aim_pose: vec![],
            trigger: vec![],
            trigger_click: vec![],
            app_menu: vec![],
            a: vec![],
            squeeze: vec![],
            squeeze_click: vec![],
            main_xy: vec![],
            main_xy_click: vec![],
            main_xy_touch: vec![],
        }
    }

    fn skeletal_input_bindings(&self, _stp: &dyn StringToPath) -> SkeletalInputBindings {
        SkeletalInputBindings {
            thumb_touch: Vec::new(),
            index_touch: Vec::new(),
            index_curl: Vec::new(),
            rest_curl: Vec::new(),
        }
    }

    fn legal_paths(&self) -> Box<[String]> {
        [].into()
    }

    fn offset_grip_pose(&self, _: Hand) -> Mat4 {
        Mat4::IDENTITY
    }
}
