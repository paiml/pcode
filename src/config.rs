use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub ai_studio_api_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            ai_studio_api_key: env::var("AI_STUDIO_API_KEY").ok(),
        }
    }
    
    pub fn has_api_key(&self) -> bool {
        self.ai_studio_api_key.is_some()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_from_env() {
        let config = Config::from_env();
        // Just verify it doesn't panic
        let _ = config.has_api_key();
    }
}