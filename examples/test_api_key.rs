use pcode::config::Config;

fn main() {
    let config = Config::from_env();
    
    println!("AI Studio API Key configured: {}", config.has_api_key());
    
    if config.has_api_key() {
        println!("API key is present (first 10 chars): {}...", 
                 config.ai_studio_api_key.as_ref().unwrap().chars().take(10).collect::<String>());
    } else {
        println!("No API key found in environment");
    }
}