use super::{SecurityError, SecurityPolicy};
use async_trait::async_trait;
use std::net::SocketAddr;

/// Cross-platform security sandbox trait
#[async_trait]
pub trait SecuritySandbox: Send + Sync {
    /// Apply security restrictions based on the policy
    fn apply_restrictions(&self, policy: &SecurityPolicy) -> Result<(), SecurityError>;

    /// Verify a manifest signature (for tool verification)
    fn verify_manifest(&self, manifest: &[u8], signature: &[u8]) -> Result<(), SecurityError>;

    /// Check if a network connection is allowed
    async fn check_network_access(&self, addr: &SocketAddr) -> Result<(), SecurityError>;

    /// Get platform name for logging
    fn platform_name(&self) -> &'static str;
}

/// Platform-specific sandbox implementations
#[cfg(target_os = "linux")]
pub type PlatformSandbox = super::linux::LinuxSandbox;

#[cfg(target_os = "macos")]
pub type PlatformSandbox = super::macos::MacOsSandbox;

#[cfg(target_os = "windows")]
pub type PlatformSandbox = super::windows::WindowsSandbox;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub type PlatformSandbox = FallbackSandbox;

/// Fallback sandbox for unsupported platforms
pub struct FallbackSandbox;

#[async_trait]
impl SecuritySandbox for FallbackSandbox {
    fn apply_restrictions(&self, _policy: &SecurityPolicy) -> Result<(), SecurityError> {
        tracing::warn!("No sandbox implementation for this platform");
        // Don't fail, just warn
        Ok(())
    }

    fn verify_manifest(&self, _manifest: &[u8], _signature: &[u8]) -> Result<(), SecurityError> {
        // Basic check - just ensure non-empty
        if _manifest.is_empty() || _signature.is_empty() {
            return Err(SecurityError::InvalidManifest(
                "Empty manifest or signature".to_string(),
            ));
        }
        Ok(())
    }

    async fn check_network_access(&self, _addr: &SocketAddr) -> Result<(), SecurityError> {
        // Allow all network access on unsupported platforms
        Ok(())
    }

    fn platform_name(&self) -> &'static str {
        "unsupported"
    }
}
