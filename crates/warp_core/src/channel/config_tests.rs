use super::*;
use crate::channel::Channel;

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
    assert!(config.session_sharing_server_url.is_none());
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

    assert_eq!(config.app_id.qualifier(), "dev");
    assert_eq!(config.app_id.organization(), "warp");
    assert_eq!(config.app_id.application_name(), "WarpOss");
    assert_eq!(config.logfile_name, "warp-oss.log");
    assert_all_endpoints_loopback(&config.server_config, &config.oz_config);
    assert!(config.telemetry_config.is_none());
    assert!(config.autoupdate_config.is_none());
    assert!(config.crash_reporting_config.is_none());
    assert!(config.mcp_static_config.is_none());
}

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

#[test]
fn oss_channel_config_is_local_only() {
    let config = ChannelConfig::local_only(AppId::new("dev", "warp", "WarpOss"), "test.log");
    assert_eq!(&*config.server_config.server_root_url, "http://localhost:0");
    assert!(config.telemetry_config.is_none());
    assert!(config.autoupdate_config.is_none());
}

#[test]
fn oss_channel_detected_correctly() {
    let channel = Channel::Oss;
    assert!(matches!(channel, Channel::Oss));
    assert!(channel != Channel::Stable);
}

#[test]
fn test_oss_channel_is_not_stable_or_dev() {
    let oss = Channel::Oss;
    assert_ne!(oss, Channel::Stable);
    assert_ne!(oss, Channel::Dev);
    assert_ne!(oss, Channel::Preview);
    assert_ne!(oss, Channel::Local);
    assert_ne!(oss, Channel::Integration);
}

#[test]
fn test_local_only_config_all_urls_are_loopback() {
    let server = WarpServerConfig::local_only();
    let oz = OzConfig::local_only();
    for url_str in [
        server.server_root_url.as_ref(),
        server.rtc_server_url.as_ref(),
        oz.oz_root_url.as_ref(),
    ] {
        let parsed = url::Url::parse(url_str).unwrap();
        let host = parsed.host_str().unwrap();
        assert!(
            host == "localhost" || host == "127.0.0.1",
            "URL {url_str} does not point at loopback"
        );
    }
}

#[test]
fn local_only_firebase_key_is_empty() {
    let config = ChannelConfig::local_only(AppId::new("dev", "warp", "WarpOss"), "test.log");
    assert!(config.server_config.firebase_auth_api_key.is_empty());
}

#[test]
fn local_only_has_no_optional_cloud_configs() {
    let config = ChannelConfig::local_only(AppId::new("dev", "warp", "WarpOss"), "test.log");
    assert!(config.telemetry_config.is_none());
    assert!(config.autoupdate_config.is_none());
    assert!(config.crash_reporting_config.is_none());
    assert!(config.mcp_static_config.is_none());
}

#[test]
fn channel_state_default_channel_is_valid() {
    use crate::channel::ChannelState;
    let ch = ChannelState::channel();
    assert!(matches!(
        ch,
        Channel::Stable
            | Channel::Preview
            | Channel::Dev
            | Channel::Local
            | Channel::Oss
            | Channel::Integration
    ));
}
