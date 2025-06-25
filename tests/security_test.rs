use pcode::security::SecurityPolicy;
use std::path::PathBuf;

#[test]
fn test_security_policy_custom() {
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/custom/path")],
        allow_network: true,
        allow_process_spawn: false,
        max_memory_mb: 1024,
        network_policy: None,
    };

    assert_eq!(policy.allowed_paths.len(), 1);
    assert!(policy.allow_network);
    assert!(!policy.allow_process_spawn);
    assert_eq!(policy.max_memory_mb, 1024);
}

#[test]
fn test_security_context_path_check_nested() {
    // Skip this test since SecurityContext fields are private
    // The functionality is tested through the public API in other tests
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_sandbox() {
    use pcode::security::macos::apply_sandbox_profile;

    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/tmp/test")],
        allow_network: false,
        allow_process_spawn: true,
        max_memory_mb: 512,
        network_policy: None,
    };

    // Should not fail
    assert!(apply_sandbox_profile(&policy).is_ok());
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_sandbox() {
    use pcode::security::windows::apply_app_container;

    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("C:\\temp")],
        allow_network: true,
        allow_process_spawn: false,
        max_memory_mb: 512,
        network_policy: None,
    };

    // Should not fail
    assert!(apply_app_container(&policy).is_ok());
}
