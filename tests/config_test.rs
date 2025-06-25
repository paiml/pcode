use pcode::config::Config;
use std::env;

#[test]
fn test_config_with_api_key() {
    env::set_var("AI_STUDIO_API_KEY", "test_key_12345");
    let config = Config::from_env();
    assert!(config.has_api_key());
    assert_eq!(config.ai_studio_api_key, Some("test_key_12345".to_string()));
    env::remove_var("AI_STUDIO_API_KEY");
}

#[test]
fn test_config_without_api_key() {
    // Save existing value if any
    let saved = env::var("AI_STUDIO_API_KEY").ok();

    // Clear the environment
    env::remove_var("AI_STUDIO_API_KEY");
    let config = Config::from_env();
    assert!(!config.has_api_key());
    assert_eq!(config.ai_studio_api_key, None);

    // Restore original value if it existed
    if let Some(key) = saved {
        env::set_var("AI_STUDIO_API_KEY", key);
    }
}

#[test]
fn test_config_default() {
    let config = Config::default();
    // Just verify it doesn't panic
    let _ = config.has_api_key();
}
