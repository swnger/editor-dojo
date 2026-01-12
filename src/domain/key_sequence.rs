/// Represents a sequence of keystrokes captured during a challenge attempt.
///
/// This value object encapsulates the raw keystroke data and provides
/// methods for formatting and displaying the sequence in a user-friendly way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySequence {
    keys: Vec<String>,
}

impl KeySequence {
    /// Creates a new KeySequence from a vector of key representations.
    pub fn new(keys: Vec<String>) -> Self {
        Self { keys }
    }

    /// Creates an empty KeySequence.
    pub fn empty() -> Self {
        Self { keys: Vec::new() }
    }

    /// Returns the total number of keystrokes in the sequence.
    pub fn count(&self) -> usize {
        self.keys.len()
    }

    /// Formats the key sequence for display with space separation.
    ///
    /// If the sequence is longer than max_length characters, it will be
    /// truncated with an ellipsis indicating how many more keys exist.
    pub fn format_for_display(&self, max_length: usize) -> String {
        if self.keys.is_empty() {
            return String::from("(no keystrokes recorded)");
        }

        let full_sequence = self.keys.join(" ");

        if full_sequence.len() <= max_length {
            full_sequence
        } else {
            // Truncate and add ellipsis with count
            let visible_keys: Vec<&str> = self.keys.iter()
                .map(|s| s.as_str())
                .take_while(|_| {
                    self.keys.iter()
                        .take_while(|&k| k != &k.to_string())
                        .map(|k| k.len() + 1) // +1 for space
                        .sum::<usize>() < max_length - 20 // Reserve space for ellipsis
                })
                .collect();

            let visible_count = visible_keys.len();
            let remaining = self.keys.len() - visible_count;

            if remaining > 0 {
                format!("{} ... ({} more keys)", visible_keys.join(" "), remaining)
            } else {
                full_sequence
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sequence() {
        let seq = KeySequence::empty();
        assert_eq!(seq.count(), 0);
        assert_eq!(seq.format_for_display(100), "(no keystrokes recorded)");
    }

    #[test]
    fn test_simple_sequence() {
        let seq = KeySequence::new(vec!["w".to_string(), "d".to_string(), "w".to_string()]);
        assert_eq!(seq.count(), 3);
        assert_eq!(seq.format_for_display(100), "w d w");
    }

    #[test]
    fn test_special_keys() {
        let seq = KeySequence::new(vec![
            "Esc".to_string(),
            ":".to_string(),
            "q".to_string(),
            "Enter".to_string(),
        ]);
        assert_eq!(seq.count(), 4);
        assert_eq!(seq.format_for_display(100), "Esc : q Enter");
    }

    #[test]
    fn test_format_for_display_no_truncation() {
        let seq = KeySequence::new(vec!["w".to_string(), "d".to_string(), "w".to_string()]);
        assert_eq!(seq.format_for_display(100), "w d w");
    }
}
