use crate::settings::CleaningSettings;
use log::debug;
use regex::RegexBuilder;

/// Apply text cleaning rules to a transcript
///
/// Returns the cleaned text, or original if cleaning is disabled or rules fail.
pub fn clean_text(input: &str, cfg: &CleaningSettings) -> String {
    if !cfg.enabled {
        return input.to_string();
    }

    if cfg.rules.is_empty() {
        debug!("Text cleaning enabled but no rules configured");
        return input.to_string();
    }

    let mut output = input.to_string();
    let mut rules_applied = 0;

    for (idx, rule) in cfg.rules.iter().enumerate() {
        let mut builder = RegexBuilder::new(&rule.pattern);

        // Apply flags if provided
        if let Some(ref flags) = rule.flags {
            if flags.contains('i') {
                builder.case_insensitive(true);
            }
            if flags.contains('m') {
                builder.multi_line(true);
            }
        }

        match builder.build() {
            Ok(re) => {
                let before_len = output.len();
                output = re.replace_all(&output, rule.replace.as_str()).into_owned();
                let after_len = output.len();

                if before_len != after_len {
                    rules_applied += 1;
                    debug!(
                        "Cleaning rule {} applied: pattern='{}' (len {} → {})",
                        idx + 1,
                        rule.pattern,
                        before_len,
                        after_len
                    );
                }
            }
            Err(e) => {
                debug!("Invalid regex in cleaning rule {}: {} - skipping", idx + 1, e);
                continue;
            }
        }
    }

    if rules_applied > 0 {
        debug!(
            "Text cleaning: {} rules applied, {} → {} chars",
            rules_applied,
            input.len(),
            output.len()
        );
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::CleaningRule;

    #[test]
    fn test_cleaning_disabled() {
        let cfg = CleaningSettings {
            enabled: false,
            profile: "basic".to_string(),
            rules: vec![],
        };
        assert_eq!(clean_text("test  text", &cfg), "test  text");
    }

    #[test]
    fn test_collapse_spaces() {
        let cfg = CleaningSettings {
            enabled: true,
            profile: "basic".to_string(),
            rules: vec![CleaningRule {
                pattern: r"\s{2,}".to_string(),
                replace: " ".to_string(),
                flags: None,
            }],
        };
        assert_eq!(clean_text("hello    world", &cfg), "hello world");
    }

    #[test]
    fn test_space_after_punctuation() {
        let cfg = CleaningSettings {
            enabled: true,
            profile: "basic".to_string(),
            rules: vec![CleaningRule {
                pattern: r"([.,!?])(\S)".to_string(),
                replace: "$1 $2".to_string(),
                flags: None,
            }],
        };
        assert_eq!(clean_text("Hello.World", &cfg), "Hello. World");
    }

    #[test]
    fn test_multiple_rules_applied_once() {
        // Test that cleaning is applied once to final merged string, not per chunk
        let cfg = CleaningSettings {
            enabled: true,
            profile: "basic".to_string(),
            rules: vec![CleaningRule {
                pattern: r"\s{2,}".to_string(),
                replace: " ".to_string(),
                flags: None,
            }],
        };

        // Simulate two chunks merged: "chunk one  " + "  chunk two"
        let merged = "chunk one    chunk two";
        let cleaned = clean_text(merged, &cfg);

        // Should collapse to single space
        assert_eq!(cleaned, "chunk one chunk two");
    }

    #[test]
    fn test_invalid_regex_skipped() {
        let cfg = CleaningSettings {
            enabled: true,
            profile: "basic".to_string(),
            rules: vec![CleaningRule {
                pattern: "[invalid".to_string(),
                replace: "".to_string(),
                flags: None,
            }],
        };
        assert_eq!(clean_text("test", &cfg), "test");
    }
}
