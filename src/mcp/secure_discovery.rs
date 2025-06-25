use super::{
    discovery::{DiscoveryError, DiscoveryStrategy, RobustToolDiscovery},
    ToolManifest,
};
use crate::security::{
    manifest::SignedManifest,
    verified_sandbox::VerifiedSandbox,
};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Secure tool discovery that verifies manifests
pub struct SecureToolDiscovery {
    inner: RobustToolDiscovery,
    sandbox: VerifiedSandbox,
}

impl Default for SecureToolDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureToolDiscovery {
    pub fn new() -> Self {
        let mut sandbox = VerifiedSandbox::new();
        
        // Load system trusted keys
        if let Err(e) = sandbox.load_system_trusted_keys() {
            warn!("Failed to load system trusted keys: {}", e);
        }
        
        Self {
            inner: RobustToolDiscovery::new(),
            sandbox,
        }
    }
    
    /// Add a trusted key for manifest verification
    pub fn add_trusted_key(&mut self, public_key: &[u8]) -> Result<(), DiscoveryError> {
        self.sandbox
            .add_trusted_key(public_key)
            .map_err(|e| DiscoveryError::Failed(format!("Failed to add trusted key: {}", e)))
    }
    
    /// Discover and verify tools
    pub async fn discover_verified(&mut self) -> Result<Vec<VerifiedTool>, DiscoveryError> {
        let manifests = self.inner.discover_all().await?;
        let mut verified_tools = Vec::new();
        
        for manifest in manifests {
            // Try to find and verify signed manifest
            match self.verify_tool_manifest(&manifest).await {
                Ok(verified) => {
                    info!("Verified tool: {} v{}", manifest.name, manifest.version);
                    verified_tools.push(verified);
                }
                Err(e) => {
                    warn!(
                        "Failed to verify tool {}: {}. Tool will be available but untrusted.",
                        manifest.name, e
                    );
                    // Still include unverified tools but mark them
                    verified_tools.push(VerifiedTool {
                        manifest,
                        signed_manifest: None,
                        is_verified: false,
                        trust_level: TrustLevel::Untrusted,
                    });
                }
            }
        }
        
        Ok(verified_tools)
    }
    
    /// Verify a tool manifest
    async fn verify_tool_manifest(
        &self,
        manifest: &ToolManifest,
    ) -> Result<VerifiedTool, DiscoveryError> {
        // Look for signed manifest file
        let manifest_paths = self.find_manifest_files(&manifest.id);
        
        for path in manifest_paths {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if let Ok(signed_manifest) = serde_json::from_str::<SignedManifest>(&content) {
                    // Verify signature
                    if signed_manifest.verify().is_ok() {
                        // Check if trusted
                        let is_trusted = self
                            .sandbox
                            .verify_signed_manifest(&signed_manifest)
                            .unwrap_or(false);
                        
                        let trust_level = if is_trusted {
                            TrustLevel::Trusted
                        } else {
                            TrustLevel::ValidSignature
                        };
                        
                        return Ok(VerifiedTool {
                            manifest: manifest.clone(),
                            signed_manifest: Some(signed_manifest),
                            is_verified: true,
                            trust_level,
                        });
                    }
                }
            }
        }
        
        // No signed manifest found
        Ok(VerifiedTool {
            manifest: manifest.clone(),
            signed_manifest: None,
            is_verified: false,
            trust_level: TrustLevel::Untrusted,
        })
    }
    
    /// Find potential manifest files for a tool
    fn find_manifest_files(&self, tool_id: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        // Check common locations
        let locations = vec![
            format!("/etc/pcode/manifests/{}.json", tool_id),
            format!("/usr/share/pcode/manifests/{}.json", tool_id),
            format!(".pcode/manifests/{}.json", tool_id),
        ];
        
        // Add user-specific paths
        if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(format!(
                "{}/.pcode/manifests/{}.json",
                home, tool_id
            )));
            paths.push(PathBuf::from(format!(
                "{}/.config/pcode/manifests/{}.json",
                home, tool_id
            )));
        }
        
        for loc in locations {
            paths.push(PathBuf::from(loc));
        }
        
        paths
    }
}

/// A tool with verification status
#[derive(Debug, Clone)]
pub struct VerifiedTool {
    pub manifest: ToolManifest,
    pub signed_manifest: Option<SignedManifest>,
    pub is_verified: bool,
    pub trust_level: TrustLevel,
}

/// Trust level for a tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Signed by a trusted key
    Trusted,
    /// Valid signature but not from trusted source
    ValidSignature,
    /// No signature or invalid signature
    Untrusted,
}

impl VerifiedTool {
    /// Get a display name with trust indicator
    pub fn display_name(&self) -> String {
        let trust_indicator = match self.trust_level {
            TrustLevel::Trusted => "✓",
            TrustLevel::ValidSignature => "?",
            TrustLevel::Untrusted => "✗",
        };
        
        format!("{} {} v{}", trust_indicator, self.manifest.name, self.manifest.version)
    }
    
    /// Check if the tool should be allowed based on policy
    pub fn is_allowed(&self, require_trusted: bool) -> bool {
        if require_trusted {
            self.trust_level == TrustLevel::Trusted
        } else {
            true
        }
    }
}

/// Discovery strategy for signed manifests
pub struct SignedManifestDiscovery {
    search_paths: Vec<PathBuf>,
}

impl SignedManifestDiscovery {
    pub fn new() -> Self {
        let mut search_paths = vec![
            PathBuf::from("/etc/pcode/manifests"),
            PathBuf::from("/usr/share/pcode/manifests"),
        ];
        
        if let Ok(home) = std::env::var("HOME") {
            search_paths.push(PathBuf::from(format!("{}/.pcode/manifests", home)));
        }
        
        Self { search_paths }
    }
}

#[async_trait]
impl DiscoveryStrategy for SignedManifestDiscovery {
    async fn discover(&self) -> Result<Vec<ToolManifest>, DiscoveryError> {
        let mut manifests = Vec::new();
        
        for path in &self.search_paths {
            if path.exists() {
                debug!("Searching for signed manifests in {:?}", path);
                
                match std::fs::read_dir(path) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            if let Some(ext) = entry.path().extension() {
                                if ext == "json" {
                                    // Try to read and parse
                                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await
                                    {
                                        if let Ok(signed) =
                                            serde_json::from_str::<SignedManifest>(&content)
                                        {
                                            // Convert to ToolManifest
                                            let manifest = ToolManifest {
                                                id: signed.manifest.id.clone(),
                                                name: signed.manifest.name.clone(),
                                                version: signed.manifest.version.clone(),
                                                description: signed.manifest.description.clone(),
                                                executable: signed.manifest.executable.clone(),
                                                tools: vec![], // Would need to convert from schema
                                            };
                                            manifests.push(manifest);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to read directory {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(manifests)
    }
    
    fn priority(&self) -> u8 {
        90 // High priority, but below MCP server
    }
    
    fn name(&self) -> &str {
        "Signed Manifest Discovery"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_secure_discovery() {
        let mut discovery = SecureToolDiscovery::new();
        
        // Should be able to discover tools (at least built-in)
        let verified_tools = discovery.discover_verified().await.unwrap();
        assert!(!verified_tools.is_empty());
        
        // Built-in tools should be untrusted (no signatures)
        let builtin = verified_tools
            .iter()
            .find(|t| t.manifest.id == "builtin")
            .unwrap();
        assert_eq!(builtin.trust_level, TrustLevel::Untrusted);
    }
    
    #[test]
    fn test_verified_tool_display() {
        let tool = VerifiedTool {
            manifest: ToolManifest {
                id: "test".to_string(),
                name: "Test Tool".to_string(),
                version: "1.0.0".to_string(),
                description: "Test".to_string(),
                executable: None,
                tools: vec![],
            },
            signed_manifest: None,
            is_verified: false,
            trust_level: TrustLevel::Untrusted,
        };
        
        assert_eq!(tool.display_name(), "✗ Test Tool v1.0.0");
        assert!(tool.is_allowed(false));
        assert!(!tool.is_allowed(true));
    }
    
    #[test]
    fn test_trust_levels() {
        let trusted = VerifiedTool {
            manifest: ToolManifest {
                id: "trusted".to_string(),
                name: "Trusted".to_string(),
                version: "1.0.0".to_string(),
                description: "".to_string(),
                executable: None,
                tools: vec![],
            },
            signed_manifest: None,
            is_verified: true,
            trust_level: TrustLevel::Trusted,
        };
        
        assert_eq!(trusted.display_name(), "✓ Trusted v1.0.0");
        assert!(trusted.is_allowed(true));
    }
}