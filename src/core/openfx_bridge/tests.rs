#[cfg(test)]
mod tests {
    use crate::core::openfx_bridge::*;
    use std::path::Path;

    #[test]
    fn test_openfx_render_is_not_reported_as_success_without_a_host() {
        let bridge = OpenFxPluginBridge::new("SapphireBlur");
        let src = vec![128u8; 16];
        let mut dst = vec![0u8; 16];

        let status = bridge.render_frame(&src, &mut dst, 2, 2, 0.0);
        assert_eq!(status, OfxStatus::Failed);
        assert!(dst.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn test_plugin_attachment_is_disabled_until_host_suites_are_implemented() {
        let error = LoadedOfxPlugin::attach(Path::new("missing.ofx"), "missing")
            .err()
            .expect("attachment must fail closed");
        assert!(error.contains("host suites are not implemented"));
    }

    #[test]
    fn test_ofx_plugin_layout_matches_the_sdk_field_order() {
        let pointer_size = std::mem::size_of::<usize>();
        assert_eq!(std::mem::offset_of!(OfxPluginRaw, plugin_api), 0);
        assert_eq!(
            std::mem::offset_of!(OfxPluginRaw, api_version),
            pointer_size
        );
        assert_eq!(
            std::mem::offset_of!(OfxPluginRaw, plugin_identifier),
            pointer_size * 2
        );
        assert_eq!(
            std::mem::offset_of!(OfxPluginRaw, plugin_version_major),
            pointer_size * 3
        );
        assert_eq!(
            std::mem::offset_of!(OfxPluginRaw, plugin_version_minor),
            pointer_size * 3 + std::mem::size_of::<std::os::raw::c_uint>()
        );
        assert_eq!(
            std::mem::offset_of!(OfxPluginRaw, set_host),
            pointer_size * 4
        );
        assert_eq!(
            std::mem::offset_of!(OfxPluginRaw, main_entry),
            pointer_size * 5
        );
        let expected_size = if pointer_size == 8 {
            pointer_size * 6
        } else {
            pointer_size * 7
        };
        assert_eq!(std::mem::size_of::<OfxPluginRaw>(), expected_size);
    }

    #[test]
    fn test_discover_finds_bundle_with_platform_binary() {
        let tmp = std::env::temp_dir().join(format!("ofx_scan_{}", std::process::id()));
        let platform_dir = if cfg!(target_os = "macos") {
            "MacOS"
        } else {
            "Linux-x86-64"
        };
        let bin_dir = tmp.join(format!("TestFX.bundle/Contents/{}", platform_dir));
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("TestFX.ofx"), b"fake").unwrap();

        let found = discover_ofx_plugins(&tmp);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "TestFX");
        assert!(found[0].binary_path.ends_with("TestFX.ofx"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_ignores_bundle_without_binary() {
        let tmp = std::env::temp_dir().join(format!("ofx_empty_{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("Empty.bundle/Contents")).unwrap();
        assert!(discover_ofx_plugins(&tmp).is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discover_missing_root_is_empty_not_panic() {
        assert!(discover_ofx_plugins(Path::new("/nonexistent/ofx/root")).is_empty());
    }

    #[test]
    fn test_probe_missing_file_is_load_error() {
        match probe_ofx_plugin(Path::new("/nonexistent/probe.ofx")) {
            OfxProbeResult::LoadError(_) => {}
            other => panic!("expected LoadError, got {other:?}"),
        }
    }

    #[test]
    fn test_probe_garbage_file_is_clean_error_not_crash() {
        let tmp = std::env::temp_dir().join(format!("ofx_garbage_{}.ofx", std::process::id()));
        std::fs::write(&tmp, b"definitely not a dylib").unwrap();
        let res = probe_ofx_plugin(&tmp);
        let _ = std::fs::remove_file(&tmp);
        assert!(
            matches!(
                res,
                OfxProbeResult::LoadError(_) | OfxProbeResult::NotOfx(_)
            ),
            "garbage must not crash: {res:?}"
        );
    }

    #[test]
    fn test_property_set_roundtrip() {
        let mut set = OfxPropertySet::new();
        assert!(set.set_int("width", 0, 1920));
        assert!(set.set_int("multi", 1, 7)); // multi-dim index
        assert!(set.set_double("scale", 0, 0.5));
        assert!(set.set_string("name", 0, "blur"));

        assert_eq!(set.get_int("width", 0), Some(1920));
        assert_eq!(set.get_int("multi", 1), Some(7));
        assert_eq!(set.get_double("scale", 0), Some(0.5));
        assert_eq!(set.get_string("name", 0), Some("blur"));
        assert_eq!(set.get_int("missing", 0), None);
        assert_eq!(
            set.get_double("width", 0),
            None,
            "type mismatch returns none"
        );
    }

    #[test]
    fn test_property_suite_c_abi_roundtrip() {
        // Drive the raw C function pointers exactly as a plugin would.
        assert_eq!(
            std::mem::size_of::<OfxPropertySuiteV1>(),
            18 * std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::offset_of!(OfxPropertySuiteV1, prop_set_pointer),
            0
        );
        assert_eq!(
            std::mem::offset_of!(OfxPropertySuiteV1, prop_set_string),
            std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::offset_of!(OfxPropertySuiteV1, prop_set_double),
            2 * std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::offset_of!(OfxPropertySuiteV1, prop_set_int),
            3 * std::mem::size_of::<usize>()
        );
        let mut set = OfxPropertySet::new();
        let handle = &mut set as *mut OfxPropertySet as *mut std::os::raw::c_void;
        let suite = property_suite_raw();
        unsafe {
            let name = b"OfxPropWidth\0";
            let rc_int = (suite.prop_set_int)(handle, name.as_ptr() as *const _, 0, 640);
            assert_eq!(rc_int, K_OFX_STAT_OK);
            let name_d = b"OfxPropScale\0";
            let rc_dbl = (suite.prop_set_double)(handle, name_d.as_ptr() as *const _, 0, 2.5);
            assert_eq!(rc_dbl, K_OFX_STAT_OK);
            let name_s = b"OfxPropName\0";
            let val = b"test\0";
            let rc_str = (suite.prop_set_string)(
                handle,
                name_s.as_ptr() as *const _,
                0,
                val.as_ptr() as *const _,
            );
            assert_eq!(rc_str, K_OFX_STAT_OK);
        }
        assert_eq!(set.get_int("OfxPropWidth", 0), Some(640));
        assert_eq!(set.get_double("OfxPropScale", 0), Some(2.5));
        assert_eq!(set.get_string("OfxPropName", 0), Some("test"));
    }
}
