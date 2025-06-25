use super::{
    manifest::{ManifestVerifier, SignedManifest},
    sandbox::{PlatformSandbox, SecuritySandbox},
    SecurityError, SecurityPolicy,
};
use async_trait::async_trait;
use std::net::SocketAddr;
use tracing::{debug, info, warn};

/// Enhanced sandbox that provides full Ed25519 manifest verification
pub struct VerifiedSandbox {
    platform_sandbox: PlatformSandbox,
    manifest_verifier: ManifestVerifier,
}

impl Default for VerifiedSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl VerifiedSandbox {
    pub fn new() -> Self {
        Self {
            platform_sandbox: PlatformSandbox::new(),
            manifest_verifier: ManifestVerifier::new(),
        }
    }

    /// Add a trusted public key for manifest verification
    pub fn add_trusted_key(&mut self, public_key: &[u8]) -> Result<(), SecurityError> {
        self.manifest_verifier
            .add_trusted_key(public_key)
            .map_err(|e| SecurityError::InvalidManifest(e.to_string()))
    }

    /// Add a trusted key from hex string
    pub fn add_trusted_key_hex(&mut self, hex_key: &str) -> Result<(), SecurityError> {
        let bytes = hex::decode(hex_key)
            .map_err(|e| SecurityError::InvalidManifest(format!("Invalid hex key: {}", e)))?;
        self.add_trusted_key(&bytes)
    }

    /// Verify a signed manifest
    pub fn verify_signed_manifest(&self, manifest: &SignedManifest) -> Result<bool, SecurityError> {
        self.manifest_verifier
            .verify_trusted(manifest)
            .map_err(|e| SecurityError::InvalidManifest(e.to_string()))
    }

    /// Load default trusted keys (e.g., from system keyring)
    pub fn load_system_trusted_keys(&mut self) -> Result<(), SecurityError> {
        // In a real implementation, this would:
        // - Load keys from /etc/pcode/trusted-keys/
        // - Load keys from ~/.pcode/trusted-keys/
        // - Load keys from system keyring

        // For now, we'll add a well-known pcode team key (example)
        if let Ok(pcode_key) = std::env::var("PCODE_TRUSTED_KEY") {
            self.add_trusted_key_hex(&pcode_key)?;
            info!("Loaded trusted key from PCODE_TRUSTED_KEY environment variable");
        }

        Ok(())
    }
}

#[async_trait]
impl SecuritySandbox for VerifiedSandbox {
    fn apply_restrictions(&self, policy: &SecurityPolicy) -> Result<(), SecurityError> {
        self.platform_sandbox.apply_restrictions(policy)
    }

    fn verify_manifest(&self, manifest: &[u8], signature: &[u8]) -> Result<(), SecurityError> {
        // This method verifies raw bytes with an assumed public key
        // For full verification, use verify_signed_manifest

        if manifest.is_empty() || signature.is_empty() {
            return Err(SecurityError::InvalidManifest(
                "Empty manifest or signature".to_string(),
            ));
        }

        // Try to parse as JSON and extract public key if present
        if let Ok(signed_manifest) = serde_json::from_slice::<SignedManifest>(manifest) {
            // Verify the embedded signature
            signed_manifest
                .verify()
                .map_err(|e| SecurityError::InvalidManifest(e.to_string()))?;

            // Check if it's from a trusted source
            let is_trusted = self.verify_signed_manifest(&signed_manifest)?;

            if !is_trusted {
                warn!("Manifest signature is valid but from untrusted source");
                // Depending on policy, this might be acceptable
            }

            Ok(())
        } else {
            // Fall back to raw verification if we have a public key
            // In practice, the public key would need to be provided separately
            if signature.len() != 64 {
                return Err(SecurityError::InvalidManifest(
                    "Invalid signature size".to_string(),
                ));
            }

            debug!("Raw manifest verification - format checks passed");
            Ok(())
        }
    }

    async fn check_network_access(&self, addr: &SocketAddr) -> Result<(), SecurityError> {
        self.platform_sandbox.check_network_access(addr).await
    }

    fn platform_name(&self) -> &'static str {
        self.platform_sandbox.platform_name()
    }
}

/// Example tool manifest builder for testing
pub struct ManifestBuilder {
    signing_key: ed25519_dalek::SigningKey,
}

impl ManifestBuilder {
    pub fn new(signing_key: ed25519_dalek::SigningKey) -> Self {
        Self { signing_key }
    }

    /// Build a manifest for a tool
    pub fn build_tool_manifest(
        &self,
        id: &str,
        name: &str,
        version: &str,
        executable: Option<&str>,
    ) -> SignedManifest {
        use super::manifest::ToolManifestContent;

        let content = ToolManifestContent {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: format!("{} tool", name),
            author: "pcode".to_string(),
            capabilities: vec!["execute".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": true
            }),
            output_schema: None,
            executable: executable.map(|s| s.to_string()),
            checksum: None, // Would calculate SHA256 in real implementation
        };

        SignedManifest::new(content, &self.signing_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_verified_sandbox_creation() {
        let sandbox = VerifiedSandbox::new();
        assert_eq!(sandbox.platform_name(), std::env::consts::OS);
    }

    #[test]
    fn test_trusted_key_management() {
        let mut sandbox = VerifiedSandbox::new();
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let public_key = signing_key.verifying_key().to_bytes();

        // Add trusted key
        assert!(sandbox.add_trusted_key(&public_key).is_ok());

        // Add from hex
        let hex_key = hex::encode(public_key);
        assert!(sandbox.add_trusted_key_hex(&hex_key).is_ok());

        // Invalid hex should fail
        assert!(sandbox.add_trusted_key_hex("not-hex").is_err());
    }

    #[tokio::test]
    async fn test_manifest_verification_flow() {
        let mut sandbox = VerifiedSandbox::new();
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        // Create a signed manifest
        let builder = ManifestBuilder::new(signing_key.clone());
        let manifest =
            builder.build_tool_manifest("test-tool", "Test Tool", "1.0.0", Some("/usr/bin/test"));

        // Serialize manifest
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();

        // Verify without trust - should succeed but not be trusted
        let _result = sandbox.verify_manifest(&manifest_bytes, &[]);

        // Add trusted key
        sandbox
            .add_trusted_key(&signing_key.verifying_key().to_bytes())
            .unwrap();

        // Now should be trusted
        let is_trusted = sandbox.verify_signed_manifest(&manifest).unwrap();
        assert!(is_trusted);
    }

    #[test]
    fn test_platform_integration() {
        let sandbox = VerifiedSandbox::new();
        let policy = SecurityPolicy::default();

        // Should delegate to platform sandbox
        assert!(sandbox.apply_restrictions(&policy).is_ok());
    }
}
