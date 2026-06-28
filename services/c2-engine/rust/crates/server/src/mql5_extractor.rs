use std::sync::OnceLock;
use regex::Regex;

const MQL5_FUNCTION_SIGNATURES: &[&str] = &[
    "void OnTick",
    "int OnInit",
    "void OnDeinit",
    "void OnStart",
    "void OnTester",
    "void OnCalculate",
];

/// Strong MQL5 keywords used to validate whether a code block is actually MQL5.
const MQL5_STRONG_INDICATORS: &[&str] = &[
    "#property",
    "OnInit",
    "OnTick",
    "OnDeinit",
    "MqlTradeRequest",
    "MqlTradeResult",
    "OrderSend",
    "PositionSelect",
    "CopyBuffer",
    "iMA",
    "iRSI",
    "iMACD",
];

fn mql5_fenced_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Catch ```mql5, ```mq5
    RE.get_or_init(|| Regex::new(r"(?s)```(?:mql5|mq5)\s*\n?(.*?)\n?\s*```").unwrap())
}

fn generic_fenced_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)```\s*\n?(.*?)\n?\s*```").unwrap())
}

/// Extract clean MQL5 source code from a raw LLM response string.
///
/// Returns `Some(clean_source)` if a valid MQL5 code block is found,
/// otherwise `None`.
///
/// Extraction priority:
/// 1. Fenced block with `mql5` language tag → clean and return.
/// 2. Fenced block without language tag → validate with MQL5 keywords → return if valid.
/// 3. No fenced blocks → scan raw text for MQL5 keyword indicators → return surrounding context if found.
///
/// Rules:
/// - Empty blocks are ignored.
/// - If multiple valid blocks exist, the first one wins.
/// - Markdown fences and language labels are stripped.
/// - No external calls, no compilation, pure text analysis.
pub fn extract_mql5_code(text: &str) -> Option<String> {
    // Phase 1: explicit ```mql5 blocks
    if let Some(caps) = mql5_fenced_re().captures(text) {
        let raw = caps.get(1)?.as_str().trim();
        if !raw.is_empty() {
            return Some(raw.to_string());
        }
    }

    // Phase 2: generic fenced blocks — validate each candidate
    for caps in generic_fenced_re().captures_iter(text) {
        let raw = caps.get(1)?.as_str().trim();
        if raw.is_empty() {
            continue;
        }
        // Skip if it has an explicit non-MQL5 language tag on the first line
        // e.g. "```python" or "```cpp" — the first line is inside the capture
        // because our generic regex doesn't include the opening tag.
        // However, some markdown renders as ```\npython\ncode... so we need to be careful.
        // We simply check strong indicators in the content.
        if looks_like_mql5(raw) {
            return Some(raw.to_string());
        }
    }

    // Phase 3: keyword fallback — only trigger if there is a REAL function signature
    // This prevents false positives like "The iMA function is used in MQL5."
    if looks_like_mql5(text) && has_mql5_function_signature(text) {
        // Heuristic: extract the first contiguous block that contains a strong indicator
        // We look for lines starting with strong indicators and gather until a blank line.
        let lines: Vec<&str> = text.lines().collect();
        let mut start: Option<usize> = None;
        let mut end: Option<usize> = None;
        for (i, line) in lines.iter().enumerate() {
            if MQL5_STRONG_INDICATORS.iter().any(|kw| line.contains(kw)) {
                if start.is_none() {
                    start = Some(i);
                }
                end = Some(i);
            } else if start.is_some() {
                // If it's a blank line, don't break immediately, just include it.
                // We'll update the end pointer whenever we see valid text.
                // To prevent grabbing trailing explanations, we only update 'end' 
                // if the line isn't just pure prose, but for a raw fallback, 
                // grabbing the rest of the block is safer than truncating.
                if !line.trim().is_empty() {
                    end = Some(i);
                }
            }
        }
        if let (Some(s), Some(e)) = (start, end) {
            let snippet = lines[s..=e].join("\n").trim().to_string();
            if !snippet.is_empty() {
                return Some(snippet);
            }
        }
    }

    None
}

/// Check whether a text snippet contains strong MQL5 indicators.
fn looks_like_mql5(text: &str) -> bool {
    MQL5_STRONG_INDICATORS.iter().any(|kw| text.contains(kw))
}

/// Check whether a text snippet contains an MQL5 event handler/function signature.
/// This is used as a stricter gate for Phase 3 fallback to avoid false positives
/// where prose mentions MQL5 keywords (e.g., "The iMA function is used in MQL5").
fn has_mql5_function_signature(text: &str) -> bool {
    MQL5_FUNCTION_SIGNATURES.iter().any(|sig| text.contains(sig))
}

#[cfg(test)]
mod tests {
    use super::extract_mql5_code;

    #[test]
    fn test_plain_text_no_code() {
        let input = "Hello";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_mql5_fenced_block() {
        let input = "```mql5\nvoid OnTick(){}\n```";
        assert_eq!(extract_mql5_code(input), Some("void OnTick(){}".to_string()));
    }

    #[test]
    fn test_mq5_fenced_block() {
        let input = "```mq5\nvoid OnTick(){}\n```";
        assert_eq!(extract_mql5_code(input), Some("void OnTick(){}".to_string()));
    }

    #[test]
    fn test_mql4_fenced_block_is_ignored() {
        let input = "```mql4\nvoid OnTick(){}\n```";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_mq4_fenced_block_is_ignored() {
        let input = "```mq4\nvoid OnTick(){}\n```";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_mql5_block_with_surrounding_text() {
        let input = "Text before.\n\n```mql5\n#property strict\n\nvoid OnTick()\n{\n}\n```\n\nText after.";
        assert_eq!(
            extract_mql5_code(input),
            Some("#property strict\n\nvoid OnTick()\n{\n}".to_string())
        );
    }

    #[test]
    fn test_python_block_is_ignored() {
        let input = "```python\nprint(\"hello\")\n```";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_cpp_block_is_ignored() {
        let input = "```cpp\n#include<iostream>\n```";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_generic_fenced_with_mql5_keywords() {
        let input = "```\n#property strict\n\nvoid OnInit()\n{\n}\n```";
        assert_eq!(
            extract_mql5_code(input),
            Some("#property strict\n\nvoid OnInit()\n{\n}".to_string())
        );
    }

    #[test]
    fn test_multiple_blocks_first_valid() {
        let input = "```mql5\ninvalid\n```\n\n```mql5\nvoid OnTick(){}\n```";
        // First block is "invalid" — it's not empty, so it wins
        assert_eq!(extract_mql5_code(input), Some("invalid".to_string()));
    }

    #[test]
    fn test_empty_mql5_block() {
        let input = "```mql5\n```";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_large_response_explanation_plus_code() {
        let input = "Here is your strategy.\n\n```mql5\n#property strict\n\nvoid OnTick()\n{\n   Print(\"Hello\");\n}\n```\n\nThis strategy prints a message.";
        assert_eq!(
            extract_mql5_code(input),
            Some("#property strict\n\nvoid OnTick()\n{\n   Print(\"Hello\");\n}".to_string())
        );
    }

    #[test]
    fn test_mql5_comments_with_markdown_characters() {
        let input = "```mql5\n// Note: **bold** and _italic_\nvoid OnTick(){}\n```";
        assert_eq!(
            extract_mql5_code(input),
            Some("// Note: **bold** and _italic_\nvoid OnTick(){}".to_string())
        );
    }

    #[test]
    fn test_random_text_with_keyword_no_fenced() {
        // "OrderSend" is a keyword but there is NO function signature like void OnTick()
        // so Phase 3 should NOT trigger. This is the fix for the false positive bug.
        let input = "The OrderSend function is used in MQL5.";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_no_fenced_no_keywords() {
        let input = "RSI works by measuring momentum.";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_pseudo_code_with_keyword_no_signature() {
        // The "What is RSI?" response had pseudo-code with iMA but no function signature
        // Phase 3 should NOT trigger because there's no void OnTick() or int OnInit()
        let input = "// Pseudo-code for RSI calculation (not actual MQL5)\ndouble avgGain = iMA(gain, period, 0, MODE_SMA);";
        assert_eq!(extract_mql5_code(input), None);
    }

    #[test]
    fn test_nested_backticks() {
        let input = "```mql5\nstring s = \"`nested`\";\nvoid OnTick(){}\n```";
        assert_eq!(
            extract_mql5_code(input),
            Some("string s = \"`nested`\";\nvoid OnTick(){}".to_string())
        );
    }

    #[test]
    fn test_mql5_with_extra_whitespace() {
        let input = "```mql5\n\n\n   #property strict   \n\n   void OnTick()\n   {\n   }\n\n\n```";
        assert_eq!(
            extract_mql5_code(input),
            Some("#property strict\n\n   void OnTick()\n   {\n   }".to_string())
        );
    }

    #[test]
    fn test_multiple_strategies_first_wins() {
        let input = r#"Strategy A:
```mql5
void StrategyA(){}
```
Strategy B:
```mql5
void StrategyB(){}
```"#;
        assert_eq!(extract_mql5_code(input), Some("void StrategyA(){}".to_string()));
    }

    #[test]
    fn test_broken_code_still_extracted() {
        let input = "```mql5\nvoid OnTick(\n```";
        // Broken code is still extracted; validation is the compiler's job
        assert_eq!(extract_mql5_code(input), Some("void OnTick(".to_string()));
    }

    #[test]
    fn test_partial_generated_code_with_fences() {
        let input = "```mql5\n#property strict\nint OnInit()\n{\n   return(INIT_SUCCEEDED);\n}\n```";
        assert_eq!(
            extract_mql5_code(input),
            Some("#property strict\nint OnInit()\n{\n   return(INIT_SUCCEEDED);\n}".to_string())
        );
    }

    #[test]
    fn test_no_mql5_tag_but_code_inside() {
        let input = "```\n#property strict\nvoid OnTick(){}\n```";
        assert_eq!(extract_mql5_code(input), Some("#property strict\nvoid OnTick(){}".to_string()));
    }
}
