use crate::input::{
    action_manifest::{PathTranslation, StringToPath},
    legacy::LegacyBindings,
    InteractionProfile,
};

pub struct Knuckles;

impl InteractionProfile for Knuckles {
    const PROFILE_PATH: &'static str = "/interaction_profiles/valve/index_controller";
    const MODEL: &'static std::ffi::CStr = c"Knuckles";
    const OPENVR_CONTROLLER_TYPE: &'static std::ffi::CStr = c"knuckles";
    const TRANSLATE_MAP: &'static [PathTranslation] = &[
        PathTranslation {
            from: "pull",
            to: "value",
            stop: false,
        },
        PathTranslation {
            from: "input/grip",
            to: "input/squeeze",
            stop: false,
        },
        PathTranslation {
            from: "squeeze/click",
            to: "squeeze/force",
            stop: true,
        },
        PathTranslation {
            from: "squeeze/grab",
            to: "squeeze/force",
            stop: true,
        },
        PathTranslation {
            from: "trackpad/click",
            to: "trackpad/force",
            stop: true,
        },
    ];

    fn legal_paths() -> Box<[String]> {
        let click_and_touch = ["input/a", "input/b", "input/trigger", "input/thumbstick"]
            .iter()
            .flat_map(|p| [format!("{p}/click"), format!("{p}/touch")]);
        let x_and_y = ["input/thumbstick", "input/trackpad"]
            .iter()
            .flat_map(|p| [format!("{p}/x"), format!("{p}/y"), p.to_string()]);
        let misc = [
            "input/squeeze/value",
            "input/squeeze/force",
            "input/trigger/value",
            "input/trackpad/force",
            "input/trackpad/touch",
            "input/grip/pose",
            "input/aim/pose",
            "output/haptic",
        ]
        .into_iter()
        .map(String::from);

        click_and_touch
            .chain(x_and_y)
            .chain(misc)
            .flat_map(|p| {
                [
                    format!("/user/hand/left/{p}"),
                    format!("/user/hand/right/{p}"),
                ]
            })
            .collect()
    }

    fn legacy_bindings(stp: impl StringToPath) -> LegacyBindings {
        LegacyBindings {
            grip_pose: stp.leftright("input/grip/pose"),
            aim_pose: stp.leftright("input/aim/pose"),
            app_menu: stp.leftright("input/b/click"),
            trigger: stp.leftright("input/trigger/click"),
            trigger_click: stp.leftright("input/trigger/value"),
            squeeze: stp.leftright("input/squeeze/value"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InteractionProfile, Knuckles};
    use crate::input::{tests::Fixture, ActionData};
    use openxr as xr;

    #[test]
    fn verify_bindings() {
        let f = Fixture::new();
        f.load_actions(c"actions.json");

        let path = Knuckles::PROFILE_PATH;
        f.verify_bindings::<bool>(
            path,
            c"/actions/set1/in/boolact",
            [
                "/user/hand/left/input/a/click".into(),
                "/user/hand/right/input/a/click".into(),
                "/user/hand/left/input/b/click".into(),
                "/user/hand/right/input/b/click".into(),
                "/user/hand/left/input/trigger/click".into(),
                "/user/hand/right/input/trigger/click".into(),
                "/user/hand/left/input/trigger/touch".into(),
                "/user/hand/right/input/trigger/touch".into(),
                "/user/hand/left/input/thumbstick/click".into(),
                "/user/hand/right/input/thumbstick/click".into(),
                "/user/hand/left/input/thumbstick/touch".into(),
                "/user/hand/right/input/thumbstick/touch".into(),
                "/user/hand/right/input/trackpad/touch".into(),
                "/user/hand/left/input/squeeze/force".into(),
                "/user/hand/left/input/trackpad/force".into(),
                "/user/hand/right/input/trackpad/force".into(),
            ],
        );

        let handle = f.get_action_handle(c"/actions/set1/in/boolact");
        let data = f.input.openxr.session_data.get();
        let actions = data.input_data.get_loaded_actions().unwrap();
        let action = actions.try_get_action(handle).unwrap();

        let ActionData::Bool(a) = action else {
            panic!("no");
        };

        let grab_data = a.grab_data.as_ref().unwrap();
        let p = f.input.openxr.instance.string_to_path(path).unwrap();
        let suggested = fakexr::get_suggested_bindings(grab_data.force_action.as_raw(), p);
        assert!(suggested.contains(&"/user/hand/right/input/squeeze/force".to_string()));

        f.verify_bindings::<f32>(
            path,
            c"/actions/set1/in/vec1act",
            [
                "/user/hand/left/input/trigger/value".into(),
                "/user/hand/right/input/trigger/value".into(),
                "/user/hand/left/input/squeeze/force".into(),
                "/user/hand/right/input/squeeze/value".into(),
            ],
        );

        f.verify_bindings::<xr::Vector2f>(
            path,
            c"/actions/set1/in/vec2act",
            [
                "/user/hand/left/input/trackpad".into(),
                "/user/hand/right/input/trackpad".into(),
                "/user/hand/left/input/thumbstick".into(),
                "/user/hand/right/input/thumbstick".into(),
            ],
        );

        f.verify_bindings::<xr::Haptic>(
            path,
            c"/actions/set1/in/vib",
            [
                "/user/hand/left/output/haptic".into(),
                "/user/hand/right/output/haptic".into(),
            ],
        );
    }
}