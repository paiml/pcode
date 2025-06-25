use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Manifest parsing error: {0}")]
    ParseError(String),

    #[error("Signature verification failed: {0}")]
    VerificationFailed(String),
}

/// Tool manifest with cryptographic signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedManifest {
    /// Manifest content
    pub manifest: ToolManifestContent,

    /// Ed25519 public key (32 bytes)
    #[serde(with = "hex_32")]
    pub public_key: [u8; 32],

    /// Ed25519 signature (64 bytes)
    #[serde(with = "hex_64")]
    pub signature: [u8; 64],
}

/// The actual manifest content that gets signed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifestContent {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<String>,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub executable: Option<String>,
    pub checksum: Option<String>, // SHA256 of executable
}

impl SignedManifest {
    /// Create a new signed manifest
    pub fn new(content: ToolManifestContent, signing_key: &SigningKey) -> Self {
        let public_key = signing_key.verifying_key().to_bytes();
        let signature = Self::sign_content(&content, signing_key);

        Self {
            manifest: content,
            public_key,
            signature: signature.to_bytes(),
        }
    }

    /// Sign manifest content
    fn sign_content(content: &ToolManifestContent, signing_key: &SigningKey) -> Signature {
        let canonical_json =
            serde_json::to_vec(content).expect("Failed to serialize manifest content");

        // Create a hash of the content for additional verification
        let mut hasher = Sha256::new();
        hasher.update(&canonical_json);
        let hash = hasher.finalize();

        debug!("Signing manifest with hash: {}", hex::encode(hash));

        signing_key.sign(&canonical_json)
    }

    /// Verify the manifest signature
    pub fn verify(&self) -> Result<(), ManifestError> {
        // Parse the public key
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|e| ManifestError::InvalidPublicKey(e.to_string()))?;

        // Parse the signature
        let signature = Signature::from_bytes(&self.signature);

        // Serialize the content in the same way it was signed
        let canonical_json = serde_json::to_vec(&self.manifest)
            .map_err(|e| ManifestError::ParseError(e.to_string()))?;

        // Verify the signature
        verifying_key
            .verify(&canonical_json, &signature)
            .map_err(|e| ManifestError::VerificationFailed(e.to_string()))?;

        info!("Manifest signature verified for {}", self.manifest.id);
        Ok(())
    }

    /// Get the manifest ID with public key fingerprint
    pub fn qualified_id(&self) -> String {
        let key_fingerprint = &hex::encode(&self.public_key[..8]);
        format!("{}@{}", self.manifest.id, key_fingerprint)
    }
}

/// Manifest verifier with trust management
pub struct ManifestVerifier {
    trusted_keys: Vec<VerifyingKey>,
}

impl Default for ManifestVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestVerifier {
    pub fn new() -> Self {
        Self {
            trusted_keys: Vec::new(),
        }
    }

    /// Add a trusted public key
    pub fn add_trusted_key(&mut self, public_key: &[u8]) -> Result<(), ManifestError> {
        if public_key.len() != 32 {
            return Err(ManifestError::InvalidPublicKey(
                "Public key must be 32 bytes".to_string(),
            ));
        }

        let key = VerifyingKey::from_bytes(public_key.try_into().unwrap())
            .map_err(|e| ManifestError::InvalidPublicKey(e.to_string()))?;

        self.trusted_keys.push(key);
        debug!("Added trusted key: {}", hex::encode(&public_key[..8]));
        Ok(())
    }

    /// Verify a manifest and check if it's from a trusted source
    pub fn verify_trusted(&self, manifest: &SignedManifest) -> Result<bool, ManifestError> {
        // First verify the signature is valid
        manifest.verify()?;

        // Then check if the key is trusted
        let verifying_key = VerifyingKey::from_bytes(&manifest.public_key)
            .map_err(|e| ManifestError::InvalidPublicKey(e.to_string()))?;

        let is_trusted = self.trusted_keys.iter().any(|k| k == &verifying_key);

        if !is_trusted {
            warn!(
                "Manifest {} signed by untrusted key {}",
                manifest.manifest.id,
                hex::encode(&manifest.public_key[..8])
            );
        }

        Ok(is_trusted)
    }

    /// Verify raw manifest bytes
    pub fn verify_raw(
        &self,
        manifest_bytes: &[u8],
        signature_bytes: &[u8],
        public_key_bytes: &[u8],
    ) -> Result<(), ManifestError> {
        if public_key_bytes.len() != 32 {
            return Err(ManifestError::InvalidPublicKey(
                "Public key must be 32 bytes".to_string(),
            ));
        }

        if signature_bytes.len() != 64 {
            return Err(ManifestError::InvalidSignature);
        }

        let verifying_key = VerifyingKey::from_bytes(public_key_bytes.try_into().unwrap())
            .map_err(|e| ManifestError::InvalidPublicKey(e.to_string()))?;

        let signature = Signature::from_bytes(signature_bytes.try_into().unwrap());

        verifying_key
            .verify(manifest_bytes, &signature)
            .map_err(|e| ManifestError::VerificationFailed(e.to_string()))?;

        Ok(())
    }
}

// Hex serialization for 32-byte arrays
mod hex_32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;

        if bytes.len() != 32 {
            return Err(serde::de::Error::custom(
                "Invalid key length: expected 32 bytes",
            ));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

// Hex serialization for 64-byte arrays
mod hex_64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;

        if bytes.len() != 64 {
            return Err(serde::de::Error::custom(
                "Invalid signature length: expected 64 bytes",
            ));
        }

        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_manifest() -> ToolManifestContent {
        ToolManifestContent {
            id: "test-tool".to_string(),
            name: "Test Tool".to_string(),
            version: "1.0.0".to_string(),
            description: "A test tool".to_string(),
            author: "Test Author".to_string(),
            capabilities: vec!["read".to_string(), "write".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            }),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string" }
                }
            })),
            executable: Some("/usr/bin/test-tool".to_string()),
            checksum: None,
        }
    }

    #[test]
    fn test_manifest_signing_and_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let content = create_test_manifest();

        let signed_manifest = SignedManifest::new(content, &signing_key);

        // Verify should succeed
        assert!(signed_manifest.verify().is_ok());

        // Check qualified ID
        let qid = signed_manifest.qualified_id();
        assert!(qid.starts_with("test-tool@"));
    }

    #[test]
    fn test_manifest_tampering_detection() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let content = create_test_manifest();

        let mut signed_manifest = SignedManifest::new(content, &signing_key);

        // Tamper with the manifest
        signed_manifest.manifest.version = "2.0.0".to_string();

        // Verification should fail
        assert!(signed_manifest.verify().is_err());
    }

    #[test]
    fn test_manifest_verifier() {
        let signing_key1 = SigningKey::generate(&mut OsRng);
        let signing_key2 = SigningKey::generate(&mut OsRng);

        let content = create_test_manifest();
        let signed_manifest = SignedManifest::new(content, &signing_key1);

        let mut verifier = ManifestVerifier::new();

        // Without trusted keys, should verify but not be trusted
        let trusted = verifier.verify_trusted(&signed_manifest).unwrap();
        assert!(!trusted);

        // Add the wrong key
        verifier
            .add_trusted_key(&signing_key2.verifying_key().to_bytes())
            .unwrap();
        let trusted = verifier.verify_trusted(&signed_manifest).unwrap();
        assert!(!trusted);

        // Add the correct key
        verifier
            .add_trusted_key(&signing_key1.verifying_key().to_bytes())
            .unwrap();
        let trusted = verifier.verify_trusted(&signed_manifest).unwrap();
        assert!(trusted);
    }

    #[test]
    fn test_raw_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifier = ManifestVerifier::new();

        let manifest_data = b"test manifest data";
        let signature = signing_key.sign(manifest_data);

        // Should verify successfully
        assert!(verifier
            .verify_raw(
                manifest_data,
                &signature.to_bytes(),
                &signing_key.verifying_key().to_bytes()
            )
            .is_ok());

        // Should fail with wrong data
        assert!(verifier
            .verify_raw(
                b"wrong data",
                &signature.to_bytes(),
                &signing_key.verifying_key().to_bytes()
            )
            .is_err());
    }

    #[test]
    fn test_manifest_serialization() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let content = create_test_manifest();
        let signed_manifest = SignedManifest::new(content, &signing_key);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&signed_manifest).unwrap();

        // Deserialize back
        let deserialized: SignedManifest = serde_json::from_str(&json).unwrap();

        // Should still verify
        assert!(deserialized.verify().is_ok());
        assert_eq!(signed_manifest.manifest.id, deserialized.manifest.id);
    }
}
