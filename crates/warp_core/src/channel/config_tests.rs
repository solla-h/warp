use super::*;

/// Asserts that every URL field points at loopback (never a real Warp host).
fn assert_all_endpoints_loopback(server_config: &WarpServerConfig, oz_config: &OzConfig) {
    for url in [
        server_config.server_root_url.as_ref(),
        server_config.rtc_server_url.as_ref(),
        oz_config.oz_root_url.as_ref(),
    ] {
        let parsed = url::Url::parse(url).expect("local-only URL should be valid");
        assert!(
            matches!(parsed.host_str(), Some("localhost") | Some("127.0.0.1")),
            "{url} should point at loopback, not a Warp server"
        );
    }
}

#[test]
fn warp_server_config_local_only_disables_cloud() {
    let config = WarpServerConfig::local_only();
    assert_all_endpoints_loopback(&config, &OzConfig::local_only());
    // Session sharing is a cloud feature; it must be disabled.
    assert!(config.session_sharing_server_url.is_none());
    // IAP and Firebase are cloud-only; their keys are blanked.
    assert!(config.iap_config.is_none());
    assert!(config.firebase_auth_api_key.is_empty());
}

#[test]
fn oz_config_local_only_has_no_workload_audience() {
    let config = OzConfig::local_only();
    assert!(config.workload_audience_url.is_none());
    assert_eq!(config.oz_root_url, "http://localhost:0");
}

#[test]
fn channel_config_local_only_disables_every_cloud_subsystem() {
    let config = ChannelConfig::local_only(AppId::new("dev", "warp", "WarpOss"), "warp-oss.log");

    // The id and log file round-trip through the constructor unchanged.
    assert_eq!(config.app_id.qualifier(), "dev");
    assert_eq!(config.app_id.organization(), "warp");
    assert_eq!(config.app_id.application_name(), "WarpOss");
    assert_eq!(config.logfile_name, "warp-oss.log");
    assert_all_endpoints_loopback(&config.server_config, &config.oz_config);
    // None of these cloud subsystems should be configured in a local-only build.
    assert!(config.telemetry_config.is_none());
    assert!(config.autoupdate_config.is_none());
    assert!(config.crash_reporting_config.is_none());
    assert!(config.mcp_static_config.is_none());
}

/// A regression guard: the production config is the known-bad "has cloud" state.
/// Keeping it around ensures `local_only` is contrasted against something real,
/// so future edits that accidentally re-enable cloud paths in `local_only` fail here.
#[test]
fn production_config_talks_to_cloud_so_local_only_does_not() {
    let prod = WarpServerConfig::production();
    let local = WarpServerConfig::local_only();

    assert!(
        prod.server_root_url.contains("warp.dev"),
        "test premise: production points at warp.dev"
    );
    assert!(
        !local.server_root_url.contains("warp.dev"),
        "local-only must never point at warp.dev"
    );
    assert!(prod.session_sharing_server_url.is_some());
    assert!(local.session_sharing_server_url.is_none());
}
