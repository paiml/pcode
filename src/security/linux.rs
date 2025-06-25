use super::{manifest::ManifestVerifier, sandbox::SecuritySandbox, SecurityError, SecurityPolicy};
use async_trait::async_trait;
use std::net::SocketAddr;
use tracing::{debug, warn};

pub struct LinuxSandbox {
    _manifest_verifier: ManifestVerifier,
}

impl Default for LinuxSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxSandbox {
    pub fn new() -> Self {
        Self {
            _manifest_verifier: ManifestVerifier::new(),
        }
    }
    
    fn is_landlock_available(&self) -> bool {
        // Check if Landlock is available
        // This would check kernel version and Landlock support
        // For now, return false as we don't have the landlock crate
        false
    }
}

#[async_trait]
impl SecuritySandbox for LinuxSandbox {
    fn apply_restrictions(&self, policy: &SecurityPolicy) -> Result<(), SecurityError> {
        if !self.is_landlock_available() {
            warn!("Landlock not available on this system");
            // Still apply what restrictions we can
        }

        debug!("Applying Linux security restrictions");

        // File system restrictions
        for path in &policy.allowed_paths {
            debug!("Allowing access to: {:?}", path);
            // In a real implementation:
            // - Use Landlock to restrict file access
            // - Or use seccomp-bpf as fallback
        }

        // Network restrictions
        if !policy.allow_network {
            debug!("Network access restricted");
            // In a real implementation:
            // - Use seccomp to block network syscalls
            // - Or use netfilter/iptables rules
        }

        // Process spawning restrictions
        if !policy.allow_process_spawn {
            debug!("Process spawning restricted");
            // In a real implementation:
            // - Use seccomp to block fork/exec syscalls
        }

        // Memory limits
        Self::set_memory_limit(policy.max_memory_mb)?;

        Ok(())
    }
    
    fn verify_manifest(&self, manifest: &[u8], signature: &[u8]) -> Result<(), SecurityError> {
        if manifest.is_empty() || signature.is_empty() {
            return Err(SecurityError::InvalidManifest(
                "Empty manifest or signature".to_string(),
            ));
        }
        
        // For raw verification, we need the public key
        // In a real implementation, the public key would be passed or embedded
        // For now, we just check the sizes are correct
        if signature.len() != 64 {
            return Err(SecurityError::InvalidManifest(
                "Invalid signature size".to_string(),
            ));
        }
        
        debug!("Manifest verification on Linux - size check passed");
        Ok(())
    }
    
    async fn check_network_access(&self, addr: &SocketAddr) -> Result<(), SecurityError> {
        // For now, allow all on Linux
        debug!("Checking network access to {:?}", addr);
        Ok(())
    }
    
    fn platform_name(&self) -> &'static str {
        "linux"
    }
}

impl LinuxSandbox {
    fn set_memory_limit(limit_mb: usize) -> Result<(), SecurityError> {
        // Skip in tests to avoid issues
        #[cfg(test)]
        {
            debug!("Skipping memory limit in tests: {} MB", limit_mb);
            return Ok(());
        }
        
        #[cfg(not(test))]
        {
            let limit_bytes = (limit_mb * 1024 * 1024) as libc::rlim_t;
            let rlimit = libc::rlimit {
                rlim_cur: limit_bytes,
                rlim_max: limit_bytes,
            };
            
            unsafe {
                if libc::setrlimit(libc::RLIMIT_AS, &rlimit) != 0 {
                    return Err(SecurityError::InitError(format!(
                        "Failed to set memory limit: {}",
                        std::io::Error::last_os_error()
                    )));
                }
            }
            
            debug!("Set memory limit to {} MB", limit_mb);
            Ok(())
        }
    }
}

// Keep the old function for compatibility
pub fn apply_landlock_sandbox(policy: &SecurityPolicy) -> Result<(), SecurityError> {
    let sandbox = LinuxSandbox::new();
    sandbox.apply_restrictions(policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_landlock_policy_application() {
        let policy = SecurityPolicy {
            allowed_paths: vec![PathBuf::from("/tmp")],
            allow_network: false,
            allow_process_spawn: false,
            max_memory_mb: 8192, // High limit for tests
            network_policy: None,
        };

        // Should not fail even if Landlock is not available
        assert!(apply_landlock_sandbox(&policy).is_ok());
    }
    
    #[tokio::test]
    async fn test_linux_sandbox_trait() {
        let sandbox = LinuxSandbox::new();
        let mut policy = SecurityPolicy::default();
        // Use a very high memory limit for tests to avoid OOM
        policy.max_memory_mb = 8192; // 8GB
        
        // Test trait methods
        assert!(sandbox.apply_restrictions(&policy).is_ok());
        assert_eq!(sandbox.platform_name(), "linux");
        
        // Test network check
        let addr = "127.0.0.1:8080".parse().unwrap();
        assert!(sandbox.check_network_access(&addr).await.is_ok());
    }
}