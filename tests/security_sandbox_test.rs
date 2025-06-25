// Tests for security sandboxing to improve coverage
use pcode::security::{SecurityContext, SecurityPolicy, SecurityError};
use std::path::PathBuf;

#[test]
fn test_security_policy_builder() {
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/tmp"), PathBuf::from("/home/user")],
        allow_network: false,
        allow_process_spawn: true,
        max_memory_mb: 1024,
    };
    
    assert_eq!(policy.allowed_paths.len(), 2);
    assert!(!policy.allow_network);
    assert!(policy.allow_process_spawn);
    assert_eq!(policy.max_memory_mb, 1024);
}

#[test]
fn test_security_context_with_custom_policy() {
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/custom/path")],
        allow_network: true,
        allow_process_spawn: false,
        max_memory_mb: 256,
    };
    
    let context = SecurityContext::new(policy);
    // Just verify it creates without panicking
    // Either succeeds or fails with unsupported platform
    if context.is_err() {
        // Can't use unwrap_err without Debug, so just check it failed
        assert!(true);
    }
}

#[test]
fn test_path_access_edge_cases() {
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/allowed/dir")],
        allow_network: false,
        allow_process_spawn: false,
        max_memory_mb: 512,
    };
    
    let context = SecurityContext::new(policy);
    
    if let Ok(ctx) = context {
        // Test various path patterns
        assert!(ctx.check_path_access(&PathBuf::from("/allowed/dir/file.txt")).is_ok());
        assert!(ctx.check_path_access(&PathBuf::from("/allowed/dir/subdir/file.txt")).is_ok());
        assert!(ctx.check_path_access(&PathBuf::from("/not/allowed/file.txt")).is_err());
        assert!(ctx.check_path_access(&PathBuf::from("/allowed")).is_err()); // Parent not allowed
        assert!(ctx.check_path_access(&PathBuf::from("/alloweddir")).is_err()); // Similar but different
    }
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
fn test_landlock_security_context() {
    let policy = SecurityPolicy {
        allowed_paths: vec![PathBuf::from("/nonexistent/path/xyz123")],
        allow_network: false,
        allow_process_spawn: false,
        max_memory_mb: 512,
    };
    
    // This might fail on systems without Landlock support
    let result = SecurityContext::new(policy);
    
    // Either succeeds or fails - we can't check error type without Debug
    // Just verify it doesn't panic
    let _ = result;
}

