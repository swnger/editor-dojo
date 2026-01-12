use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::domain::{KeySequence, Recording};
use super::cast_parser::CastParser;

/// Trait for recording challenge attempts.
pub trait Recorder {
    /// Starts recording and spawns the editor with the given file.
    ///
    /// Returns a handle to the spawned process.
    fn start_recording(&mut self, file_path: &Path, output_path: &Path) -> Result<Child>;

    /// Stops the recording and parses the output to create a Recording.
    ///
    /// This should be called after the editor process has exited.
    fn finalize_recording(&self, output_path: &Path) -> Result<Recording>;

    /// Checks if asciinema is installed and available.
    ///
    /// This is a static method, not available on trait objects.
    fn is_available() -> bool where Self: Sized;
}

/// Implementation of Recorder using asciinema.
pub struct AsciinemaRecorder {
    editor_command: String,
}

impl AsciinemaRecorder {
    /// Creates a new AsciinemaRecorder with the specified editor command.
    ///
    /// The editor_command should be the base command (e.g., "hx" for Helix).
    pub fn new(editor_command: impl Into<String>) -> Self {
        Self {
            editor_command: editor_command.into(),
        }
    }

    /// Ensures the recordings directory exists.
    fn ensure_recordings_dir() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;

        let recordings_dir = PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("editor-dojo")
            .join("recordings");

        if !recordings_dir.exists() {
            std::fs::create_dir_all(&recordings_dir)
                .with_context(|| format!("Failed to create recordings directory: {}", recordings_dir.display()))?;
        }

        Ok(recordings_dir)
    }

    /// Generates a unique recording filename for a challenge.
    pub fn generate_recording_path(challenge_id: &str) -> Result<PathBuf> {
        let recordings_dir = Self::ensure_recordings_dir()?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("Failed to get system time")?
            .as_secs();

        let filename = format!("challenge-{}-{}.cast", challenge_id, timestamp);
        Ok(recordings_dir.join(filename))
    }
}

impl Recorder for AsciinemaRecorder {
    fn start_recording(&mut self, file_path: &Path, output_path: &Path) -> Result<Child> {
        // Build the command to record: asciinema rec --overwrite <output> -c "hx <file>"
        let editor_command = format!("{} {}", self.editor_command, file_path.display());

        let child = Command::new("asciinema")
            .arg("rec")
            .arg("--overwrite")
            .arg(output_path)
            .arg("-c")
            .arg(editor_command)
            .spawn()
            .context("Failed to start asciinema recording")?;

        Ok(child)
    }

    fn finalize_recording(&self, output_path: &Path) -> Result<Recording> {
        // Parse the .cast file to extract keystrokes
        let key_sequence = CastParser::parse(output_path)
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to parse recording: {}", e);
                KeySequence::empty()
            });

        Ok(Recording::new(output_path.to_path_buf(), key_sequence))
    }

    fn is_available() -> bool {
        Command::new("asciinema")
            .arg("--version")
            .output()
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asciinema_recorder_creation() {
        let recorder = AsciinemaRecorder::new("hx");
        assert_eq!(recorder.editor_command, "hx");
    }

    #[test]
    fn test_generate_recording_path() {
        let path = AsciinemaRecorder::generate_recording_path("test-01");
        assert!(path.is_ok());

        if let Ok(path) = path {
            assert!(path.to_string_lossy().contains("recordings"));
            assert!(path.to_string_lossy().contains("challenge-test-01"));
            assert!(path.extension().unwrap() == "cast");
        }
    }

    #[test]
    fn test_is_available() {
        // This will depend on whether asciinema is actually installed
        // Just ensure the method doesn't panic
        let _ = AsciinemaRecorder::is_available();
    }
}
