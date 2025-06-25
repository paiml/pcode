// Tests for token estimation to improve coverage  
use pcode::token_estimation::Tokenizer;

#[test]
fn test_token_estimation_edge_cases() {
    let tokenizer = Tokenizer::new();
    
    // Empty string
    assert_eq!(tokenizer.estimate_tokens(""), 0);
    
    // Whitespace only
    assert_eq!(tokenizer.estimate_tokens("   \n\t  "), 0);
    
    // Single character
    assert_eq!(tokenizer.estimate_tokens("a"), 1);
    
    // Numbers
    let count = tokenizer.estimate_tokens("12345 67890");
    assert!(count >= 2);
    
    // Punctuation
    let count = tokenizer.estimate_tokens("Hello, world! How are you?");
    assert!(count >= 5);
    
    // Mixed content
    let count = tokenizer.estimate_tokens("The price is $99.99 (on sale!)");
    assert!(count >= 6);
}

#[test]
fn test_fast_estimation_accuracy() {
    let tokenizer = Tokenizer::new();
    
    // Fast estimation should be close to regular estimation
    let text = "This is a test of the token estimation system. It should work well.";
    
    let regular = tokenizer.estimate_tokens(text);
    let fast = tokenizer.estimate_tokens_fast(text);
    
    // Fast estimation might be slightly different but should be close
    let diff = (regular as i32 - fast as i32).abs();
    assert!(diff <= regular as i32 / 2); // Within 50%
}

#[test]
fn test_token_estimation_unicode() {
    let tokenizer = Tokenizer::new();
    
    // Unicode text
    let count = tokenizer.estimate_tokens("Hello 世界 🌍");
    assert!(count >= 3);
    
    // Emojis
    let count = tokenizer.estimate_tokens("👋 😊 🚀");
    assert!(count >= 3);
    
    // Mixed scripts
    let count = tokenizer.estimate_tokens("English русский 日本語");
    assert!(count >= 3);
}

#[test]
fn test_token_estimation_special_cases() {
    let tokenizer = Tokenizer::new();
    
    // URLs
    let count = tokenizer.estimate_tokens("Visit https://example.com/path?query=value");
    assert!(count >= 2);
    
    // Code snippets
    let count = tokenizer.estimate_tokens("fn main() { println!(\"Hello\"); }");
    assert!(count >= 6);
    
    // Repeated characters
    let count = tokenizer.estimate_tokens("aaaaaaaaaa");
    assert!(count >= 1);
    
    // Long words
    let count = tokenizer.estimate_tokens("supercalifragilisticexpialidocious");
    assert!(count >= 1);
}

#[test]
fn test_token_estimator_consistency() {
    let tokenizer = Tokenizer::new();
    
    // Same input should give same output
    let text = "The quick brown fox jumps over the lazy dog";
    let count1 = tokenizer.estimate_tokens(text);
    let count2 = tokenizer.estimate_tokens(text);
    assert_eq!(count1, count2);
    
    // Fast estimate should also be consistent
    let fast1 = tokenizer.estimate_tokens_fast(text);
    let fast2 = tokenizer.estimate_tokens_fast(text);
    assert_eq!(fast1, fast2);
}

#[test]
fn test_tokenizer_instance() {
    // Test singleton instance
    let instance1 = Tokenizer::instance();
    let instance2 = Tokenizer::instance();
    
    // Both should give same results
    let text = "test text";
    assert_eq!(instance1.estimate_tokens(text), instance2.estimate_tokens(text));
}