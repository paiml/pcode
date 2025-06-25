use std::path::PathBuf;
use tracing::debug;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub mod sandbox;

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Failed to initialize sandbox: {0}")]
    InitError(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Unsupported platform")]
    UnsupportedPlatform,
    
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    
    #[error("Network access denied: {0}")]
    NetworkAccessDenied(String),
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub allowed_paths: Vec<PathBuf>,
    pub allow_network: bool,
    pub allow_process_spawn: bool,
    pub max_memory_mb: usize,
    pub network_policy: Option<NetworkPolicy>,
}

#[derive(Debug, Clone)]
pub struct NetworkPolicy {
    pub allowed_domains: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub allowed_protocols: Vec<Protocol>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Tcp,
    Udp,
    Http,
    Https,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allowed_paths: vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))],
            allow_network: false,
            allow_process_spawn: false,
            max_memory_mb: 2048, // 2GB default
            network_policy: None,
        }
    }
}

pub struct SecurityContext {
    policy: SecurityPolicy,
}

impl SecurityContext {
    pub fn new(policy: SecurityPolicy) -> Result<Self, SecurityError> {
        debug!("Initializing security context with policy: {:?}", policy);

        let context = Self { policy };
        context.apply_sandbox()?;

        Ok(context)
    }

    #[cfg(target_os = "linux")]
    fn apply_sandbox(&self) -> Result<(), SecurityError> {
        linux::apply_landlock_sandbox(&self.policy)
    }

    #[cfg(target_os = "macos")]
    fn apply_sandbox(&self) -> Result<(), SecurityError> {
        macos::apply_sandbox_profile(&self.policy)
    }

    #[cfg(target_os = "windows")]
    fn apply_sandbox(&self) -> Result<(), SecurityError> {
        windows::apply_app_container(&self.policy)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn apply_sandbox(&self) -> Result<(), SecurityError> {
        tracing::warn!("No sandbox implementation for this platform");
        Err(SecurityError::UnsupportedPlatform)
    }

    pub fn check_path_access(&self, path: &PathBuf) -> Result<(), SecurityError> {
        for allowed_path in &self.policy.allowed_paths {
            if path.starts_with(allowed_path) {
                return Ok(());
            }
        }

        Err(SecurityError::PermissionDenied(format!(
            "Access to path {:?} is not allowed",
            path
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = SecurityPolicy::default();
        assert!(!policy.allow_network);
        assert!(!policy.allow_process_spawn);
        assert_eq!(policy.max_memory_mb, 2048);
    }

    #[test]
    fn test_path_access_check() {
        let policy = SecurityPolicy {
            allowed_paths: vec![PathBuf::from("/tmp/test")],
            ..Default::default()
        };

        let context = SecurityContext { policy };

        assert!(context
            .check_path_access(&PathBuf::from("/tmp/test/file.txt"))
            .is_ok());
        assert!(context
            .check_path_access(&PathBuf::from("/etc/passwd"))
            .is_err());
    }
}
