use std::path::PathBuf;
use super::key_sequence::KeySequence;

/// Represents a recording of a challenge attempt.
///
/// This value object encapsulates the asciinema recording file path
/// and the extracted keystroke sequence.
#[derive(Debug, Clone)]
pub struct Recording {
    /// Path to the .cast file
    file_path: PathBuf,

    /// Extracted keystroke sequence
    key_sequence: KeySequence,
}

impl Recording {
    /// Creates a new Recording with a file path and key sequence.
    pub fn new(file_path: PathBuf, key_sequence: KeySequence) -> Self {
        Self {
            file_path,
            key_sequence,
        }
    }


    /// Returns a reference to the key sequence.
    pub fn key_sequence(&self) -> &KeySequence {
        &self.key_sequence
    }

    /// Returns the total number of keystrokes in the recording.
    pub fn keystroke_count(&self) -> usize {
        self.key_sequence.count()
    }

    /// Returns the file path as a string for display purposes.
    pub fn file_path_display(&self) -> String {
        self.file_path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_creation() {
        let path = PathBuf::from("/tmp/test.cast");
        let seq = KeySequence::new(vec!["w".to_string(), "d".to_string()]);
        let recording = Recording::new(path.clone(), seq);

        assert_eq!(recording.keystroke_count(), 2);
    }

    #[test]
    fn test_file_path_display() {
        let path = PathBuf::from("/home/user/.local/share/editor-dojo/recordings/test.cast");
        let seq = KeySequence::empty();
        let recording = Recording::new(path, seq);

        assert!(recording.file_path_display().contains("recordings/test.cast"));
    }
}
