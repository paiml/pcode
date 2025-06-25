use std::sync::OnceLock;
use tracing::debug;

// Include the generated lookup table
static TOKEN_TABLE: [u16; 131072] = include!(concat!(env!("OUT_DIR"), "/token_table.rs"));

static TOKENIZER: OnceLock<CompactTokenCounter> = OnceLock::new();

// Minimal BPE rules for fallback
struct BpeRuleset {
    merges: Vec<(Vec<u8>, Vec<u8>, u16)>, // (left, right, token_count)
}

impl BpeRuleset {
    fn minimal() -> Self {
        // Include only the most common merges
        Self {
            merges: vec![
                (b"th".to_vec(), b"e".to_vec(), 1),
                (b"in".to_vec(), b"g".to_vec(), 1),
                (b"er".to_vec(), b"".to_vec(), 1),
                (b"on".to_vec(), b"".to_vec(), 1),
                (b"at".to_vec(), b"".to_vec(), 1),
                (b"en".to_vec(), b"".to_vec(), 1),
                (b"ed".to_vec(), b"".to_vec(), 1),
                (b"to".to_vec(), b"".to_vec(), 1),
                (b"it".to_vec(), b"".to_vec(), 1),
                (b"ou".to_vec(), b"".to_vec(), 1),
                (b"ea".to_vec(), b"".to_vec(), 1),
                (b"hi".to_vec(), b"".to_vec(), 1),
                (b"is".to_vec(), b"".to_vec(), 1),
                (b"or".to_vec(), b"".to_vec(), 1),
                (b"ti".to_vec(), b"".to_vec(), 1),
                (b"as".to_vec(), b"".to_vec(), 1),
                (b"te".to_vec(), b"".to_vec(), 1),
                (b"et".to_vec(), b"".to_vec(), 1),
                (b"ng".to_vec(), b"".to_vec(), 1),
                (b"of".to_vec(), b"".to_vec(), 1),
            ],
        }
    }

    fn tokenize_at(&self, bytes: &[u8]) -> (usize, usize) {
        // Simple BPE tokenization
        if bytes.is_empty() {
            return (0, 0);
        }

        // Try to match common merges
        for (left, right, _) in &self.merges {
            if bytes.starts_with(left)
                && (right.is_empty() || bytes[left.len()..].starts_with(right))
            {
                return (1, left.len() + right.len());
            }
        }

        // Default: one token per 4 characters on average
        let char_count = std::cmp::min(4, bytes.len());
        (1, char_count)
    }
}

pub struct CompactTokenCounter {
    // 256KB lookup table for common patterns
    pattern_table: &'static [u16; 131072], // 2^17 entries
    // Simple BPE rules for fallback
    bpe_rules: BpeRuleset,
}

impl Default for CompactTokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl CompactTokenCounter {
    pub fn new() -> Self {
        Self {
            pattern_table: &TOKEN_TABLE,
            bpe_rules: BpeRuleset::minimal(),
        }
    }

    pub fn instance() -> &'static Self {
        TOKENIZER.get_or_init(Self::new)
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        // Skip whitespace-only strings
        if text.trim().is_empty() {
            return 0;
        }

        let mut tokens = 0;
        let bytes = text.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            // Skip whitespace
            if bytes[i].is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // Try to match longest pattern in lookup table
            let mut matched = false;

            for len in (1..=8).rev() {
                if i + len <= bytes.len() {
                    // Simple hash function (same as in build.rs)
                    let hash = bytes[i..i + len]
                        .iter()
                        .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64))
                        as usize;
                    let index = hash & 0x1FFFF; // Mask to 17 bits

                    if self.pattern_table[index] != 0 {
                        tokens += self.pattern_table[index] as usize;
                        i += len;
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                // Fallback to simple BPE
                let (token_count, bytes_consumed) = self.bpe_rules.tokenize_at(&bytes[i..]);
                tokens += token_count;
                i += bytes_consumed;
            }
        }

        debug!(
            "Estimated {} tokens for text of length {}",
            tokens,
            text.len()
        );
        tokens
    }

    pub fn estimate_tokens(&self, text: &str) -> usize {
        self.count_tokens(text)
    }

    pub fn estimate_tokens_fast(&self, text: &str) -> usize {
        // Fast approximation based on character count
        // Average of 4 characters per token
        (text.len() as f32 / 4.0).ceil() as usize
    }
}

// Re-export the old Tokenizer name for compatibility
pub type Tokenizer = CompactTokenCounter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let tokenizer = CompactTokenCounter::new();

        assert_eq!(tokenizer.estimate_tokens(""), 0);
        assert!(tokenizer.estimate_tokens("the") >= 1);
        assert!(tokenizer.estimate_tokens("the cat") >= 2);
        assert!(tokenizer.estimate_tokens("hello world") > 0);
    }

    #[test]
    fn test_fast_estimation() {
        let tokenizer = CompactTokenCounter::new();
        let text = "This is a longer text for testing fast token estimation";

        let fast = tokenizer.estimate_tokens_fast(text);
        let accurate = tokenizer.estimate_tokens(text);

        // Fast should be within reasonable range of accurate
        assert!(fast > 0);
        assert!(accurate > 0);
        assert!((fast as f32 / accurate as f32) > 0.5);
        assert!((fast as f32 / accurate as f32) < 2.0);
    }

    #[test]
    fn test_common_tokens() {
        let tokenizer = CompactTokenCounter::new();

        // These should be recognized by our lookup table
        assert_eq!(tokenizer.estimate_tokens("fn"), 1);
        assert_eq!(tokenizer.estimate_tokens("let"), 1);
        assert_eq!(tokenizer.estimate_tokens("mut"), 1);
        assert_eq!(tokenizer.estimate_tokens("if"), 1);
        assert_eq!(tokenizer.estimate_tokens("else"), 1);
    }
}
