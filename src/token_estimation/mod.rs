use phf::phf_map;
use std::sync::OnceLock;
use tracing::debug;

// Static perfect hash table for token estimation
// In production, this would be generated at build time
static TOKEN_MAP: phf::Map<&'static str, u32> = phf_map! {
    "the" => 1,
    "of" => 1,
    "to" => 1,
    "and" => 1,
    "a" => 1,
    "in" => 1,
    "is" => 1,
    "it" => 1,
    "for" => 1,
    "with" => 1,
    "was" => 1,
    "on" => 1,
    "are" => 1,
    "as" => 1,
    "by" => 1,
    "at" => 1,
    "from" => 1,
    "that" => 1,
    "this" => 1,
    "be" => 1,
};

static TOKENIZER: OnceLock<Tokenizer> = OnceLock::new();

pub struct Tokenizer {
    avg_chars_per_token: f32,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            avg_chars_per_token: 4.0,
        }
    }

    pub fn instance() -> &'static Self {
        TOKENIZER.get_or_init(Self::new)
    }

    pub fn estimate_tokens(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        let mut token_count = 0;
        let words: Vec<&str> = text.split_whitespace().collect();

        for word in &words {
            let lower = word.to_lowercase();
            if TOKEN_MAP.contains_key(lower.as_str()) {
                token_count += TOKEN_MAP[lower.as_str()] as usize;
            } else {
                // Estimate based on character count
                token_count += (word.len() as f32 / self.avg_chars_per_token).ceil() as usize;
            }
        }

        debug!(
            "Estimated {} tokens for text of length {}",
            token_count,
            text.len()
        );
        token_count
    }

    pub fn estimate_tokens_fast(&self, text: &str) -> usize {
        // Fast approximation based on character count
        (text.len() as f32 / self.avg_chars_per_token).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let tokenizer = Tokenizer::new();

        assert_eq!(tokenizer.estimate_tokens(""), 0);
        assert_eq!(tokenizer.estimate_tokens("the"), 1);
        assert_eq!(tokenizer.estimate_tokens("the cat"), 2);
        assert!(tokenizer.estimate_tokens("hello world") > 0);
    }

    #[test]
    fn test_fast_estimation() {
        let tokenizer = Tokenizer::new();
        let text = "This is a longer text for testing fast token estimation";

        let fast = tokenizer.estimate_tokens_fast(text);
        let accurate = tokenizer.estimate_tokens(text);

        // Fast should be within 50% of accurate
        assert!((fast as f32 / accurate as f32) > 0.5);
        assert!((fast as f32 / accurate as f32) < 1.5);
    }
}
