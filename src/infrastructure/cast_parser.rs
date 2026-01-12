use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::domain::KeySequence;

/// Parser for asciinema .cast files.
///
/// Extracts keystroke data from the recording to build a human-readable
/// key sequence.
pub struct CastParser;

impl CastParser {
    /// Parses a .cast file and extracts the keystroke sequence.
    ///
    /// Returns a KeySequence containing all input events in order.
    pub fn parse(file_path: &Path) -> Result<KeySequence> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open cast file: {}", file_path.display()))?;

        let reader = BufReader::new(file);
        let mut keys = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.with_context(|| {
                format!("Failed to read line {} from cast file", line_num + 1)
            })?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // First line is the header (metadata), skip it
            if line_num == 0 {
                continue;
            }

            // Parse the event line
            match Self::parse_event(&line) {
                Ok(Some(key)) => keys.push(key),
                Ok(None) => {
                    // Not an input event, skip
                }
                Err(e) => {
                    // Log warning but continue parsing
                    eprintln!("Warning: Failed to parse event at line {}: {}", line_num + 1, e);
                }
            }
        }

        Ok(KeySequence::new(keys))
    }

    /// Parses a single event line from the .cast file.
    ///
    /// Returns Some(key) if this is an input event, None otherwise.
    fn parse_event(line: &str) -> Result<Option<String>> {
        let event: Value = serde_json::from_str(line)
            .with_context(|| "Failed to parse event JSON")?;

        // Event format: [timestamp, event_type, data]
        let event_array = event
            .as_array()
            .context("Event is not an array")?;

        if event_array.len() < 3 {
            return Ok(None);
        }

        let event_type = event_array[1]
            .as_str()
            .context("Event type is not a string")?;

        // Only process input events
        if event_type != "i" {
            return Ok(None);
        }

        let data = event_array[2]
            .as_str()
            .context("Event data is not a string")?;

        // Parse the input data into human-readable keys
        Ok(Some(Self::parse_input_data(data)))
    }

    /// Converts raw input data into human-readable key representation.
    fn parse_input_data(data: &str) -> String {
        if data.is_empty() {
            return String::new();
        }

        // Handle multi-character sequences
        if data.len() > 1 {
            return Self::parse_escape_sequence(data);
        }

        // Single character - safe to unwrap since we verified len == 1
        let ch = data.chars().next().expect("Data should have at least one character");
        Self::char_to_key_name(ch)
    }

    /// Converts a single character to its key representation.
    fn char_to_key_name(ch: char) -> String {
        match ch {
            '\n' | '\r' => "Enter".to_string(),
            '\x1b' => "Esc".to_string(),
            ' ' => "Space".to_string(),
            '\t' => "Tab".to_string(),
            '\x7f' => "Backspace".to_string(),
            '\x01'..='\x1a' => {
                // Ctrl-a through Ctrl-z
                let letter = ((ch as u8 - 1) + b'a') as char;
                format!("Ctrl-{}", letter)
            }
            c if c.is_ascii_control() => {
                // Other control characters - show as hex
                format!("<0x{:02x}>", c as u8)
            }
            c => c.to_string(),
        }
    }

    /// Parses escape sequences and multi-character inputs.
    fn parse_escape_sequence(data: &str) -> String {
        // Check for common escape sequences
        if data.starts_with('\x1b') {
            // ESC-based sequences
            if data.len() == 1 {
                return "Esc".to_string();
            }

            // Arrow keys and other escape sequences
            match data {
                "\x1b[A" => return "Up".to_string(),
                "\x1b[B" => return "Down".to_string(),
                "\x1b[C" => return "Right".to_string(),
                "\x1b[D" => return "Left".to_string(),
                "\x1b[H" => return "Home".to_string(),
                "\x1b[F" => return "End".to_string(),
                "\x1b[3~" => return "Delete".to_string(),
                "\x1b[2~" => return "Insert".to_string(),
                "\x1b[5~" => return "PageUp".to_string(),
                "\x1b[6~" => return "PageDown".to_string(),
                _ => {
                    // Alt combinations: Esc followed by a character
                    if data.len() == 2 {
                        // Safe to unwrap since we verified len == 2
                        let ch = data.chars().nth(1).expect("Data should have second character");
                        if ch.is_alphanumeric() {
                            return format!("Alt-{}", ch);
                        }
                    }
                }
            }
        }

        // If we can't parse it as a known sequence, handle it character by character
        // and join the results
        data.chars()
            .map(|ch| Self::char_to_key_name(ch))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_key_name() {
        assert_eq!(CastParser::char_to_key_name('a'), "a");
        assert_eq!(CastParser::char_to_key_name('Z'), "Z");
        assert_eq!(CastParser::char_to_key_name('1'), "1");
        assert_eq!(CastParser::char_to_key_name('\n'), "Enter");
        assert_eq!(CastParser::char_to_key_name('\r'), "Enter");
        assert_eq!(CastParser::char_to_key_name('\x1b'), "Esc");
        assert_eq!(CastParser::char_to_key_name(' '), "Space");
        assert_eq!(CastParser::char_to_key_name('\t'), "Tab");
        assert_eq!(CastParser::char_to_key_name('\x7f'), "Backspace");
        assert_eq!(CastParser::char_to_key_name('\x03'), "Ctrl-c"); // Ctrl-C
        assert_eq!(CastParser::char_to_key_name('\x04'), "Ctrl-d"); // Ctrl-D
    }

    #[test]
    fn test_parse_escape_sequences() {
        assert_eq!(CastParser::parse_escape_sequence("\x1b[A"), "Up");
        assert_eq!(CastParser::parse_escape_sequence("\x1b[B"), "Down");
        assert_eq!(CastParser::parse_escape_sequence("\x1b[C"), "Right");
        assert_eq!(CastParser::parse_escape_sequence("\x1b[D"), "Left");
        assert_eq!(CastParser::parse_escape_sequence("\x1b[3~"), "Delete");
        assert_eq!(CastParser::parse_escape_sequence("\x1ba"), "Alt-a");
        assert_eq!(CastParser::parse_escape_sequence("\x1bf"), "Alt-f");
    }

    #[test]
    fn test_parse_input_data() {
        assert_eq!(CastParser::parse_input_data("w"), "w");
        assert_eq!(CastParser::parse_input_data("d"), "d");
        assert_eq!(CastParser::parse_input_data(":"), ":");
        assert_eq!(CastParser::parse_input_data("\n"), "Enter");
        assert_eq!(CastParser::parse_input_data("\x1b"), "Esc");
        assert_eq!(CastParser::parse_input_data("\x1b[A"), "Up");
    }
}
