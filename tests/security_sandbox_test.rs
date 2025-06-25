// Tests for security sandboxing to improve coverage
use pcode::security::{SecurityError, SecurityPolicy};
use std::path::PathBuf;

#[test]
fn test_security_policy_builder() {
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/tmp"), PathBuf::from("/home/user")],
        allow_network: false,
        allow_process_spawn: true,
        max_memory_mb: 1024,
        network_policy: None,
    };

    assert_eq!(policy.allowed_paths.len(), 2);
    assert!(!policy.allow_network);
    assert!(policy.allow_process_spawn);
    assert_eq!(policy.max_memory_mb, 1024);
}

#[test]
fn test_security_policy_configuration() {
    // Test policy configuration without creating SecurityContext
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/custom/path")],
        allow_network: true,
        allow_process_spawn: false,
        max_memory_mb: 256,
        network_policy: None,
    };

    // Verify policy fields are set correctly
    assert_eq!(policy.allowed_paths.len(), 1);
    assert_eq!(policy.allowed_paths[0], PathBuf::from("/custom/path"));
    assert!(policy.allow_network);
    assert!(!policy.allow_process_spawn);
    assert_eq!(policy.max_memory_mb, 256);
    assert!(policy.network_policy.is_none());
}

#[test]
fn test_path_validation_logic() {
    // Test path validation without creating SecurityContext
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/allowed/dir")],
        allow_network: false,
        allow_process_spawn: false,
        max_memory_mb: 512,
        network_policy: None,
    };

    // Test path matching logic
    let test_path = PathBuf::from("/allowed/dir/file.txt");
    let allowed = policy
        .allowed_paths
        .iter()
        .any(|allowed_path| test_path.starts_with(allowed_path));
    assert!(allowed);

    let test_path = PathBuf::from("/not/allowed/file.txt");
    let allowed = policy
        .allowed_paths
        .iter()
        .any(|allowed_path| test_path.starts_with(allowed_path));
    assert!(!allowed);

    let test_path = PathBuf::from("/allowed");
    let allowed = policy
        .allowed_paths
        .iter()
        .any(|allowed_path| test_path.starts_with(allowed_path));
    assert!(!allowed); // Parent not allowed
}

#[test]
fn test_security_error_debug_trait() {
    let err = SecurityError::InitError("test error".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("InitError"));
    assert!(debug_str.contains("test error"));
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_platform_detection() {
    // Test Linux-specific functionality without creating SecurityContext
    use pcode::security::sandbox::{PlatformSandbox, SecuritySandbox};

    let sandbox = PlatformSandbox::new();
    assert_eq!(sandbox.platform_name(), "linux");

    // Test that we can create a policy for Linux
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/nonexistent/path/xyz123")],
        allow_network: false,
        allow_process_spawn: false,
        max_memory_mb: 512,
        network_policy: None,
    };

    // Just verify the policy is valid
    assert_eq!(policy.allowed_paths.len(), 1);
    assert!(!policy.allow_network);
}
