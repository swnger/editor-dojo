use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use anyhow::Result;

use crate::application::validator::SolutionValidator;
use crate::domain::{Challenge, Solution};
use crate::infrastructure::recorder::Recorder;

/// Trait for spawning and managing an editor process
pub trait EditorSpawner {
    /// Spawns an editor with the given file path
    fn spawn(&mut self, file_path: &Path) -> Result<()>;

    /// Terminates the editor process gracefully
    fn terminate(&mut self) -> Result<()>;

    /// Checks if the editor is still running
    fn is_running(&self) -> bool;
}

/// Trait for watching file changes
pub trait FileWatcher {
    /// Starts watching the file and sends notifications on the channel
    fn watch(&mut self, file_path: &Path, tx: mpsc::Sender<()>) -> Result<()>;

    /// Stops watching the file
    fn stop(&mut self) -> Result<()>;
}

/// Trait for file system operations
pub trait FileSystem {
    /// Creates a temporary file with the given content
    fn create_temp_file(&self, content: &str) -> Result<PathBuf>;

    /// Reads the content of a file
    fn read_file(&self, path: &Path) -> Result<String>;

    /// Cleans up the temporary file
    fn cleanup(&self, path: &Path) -> Result<()>;
}

/// Orchestrates the challenge execution flow
///
/// This is the main application service that coordinates:
/// - File creation
/// - Editor spawning
/// - File watching
/// - Solution validation
/// - Timing
/// - Recording (optional)
pub struct ChallengeRunner<E, W, F>
where
    E: EditorSpawner,
    W: FileWatcher,
    F: FileSystem,
{
    editor: E,
    watcher: W,
    filesystem: F,
    validator: SolutionValidator,
    recorder: Option<Box<dyn Recorder>>,
}

impl<E, W, F> ChallengeRunner<E, W, F>
where
    E: EditorSpawner,
    W: FileWatcher,
    F: FileSystem,
{
    pub fn new(editor: E, watcher: W, filesystem: F) -> Self {
        Self {
            editor,
            watcher,
            filesystem,
            validator: SolutionValidator::new(),
            recorder: None,
        }
    }

    pub fn with_recorder(mut self, recorder: Box<dyn Recorder>) -> Self {
        self.recorder = Some(recorder);
        self
    }

    /// Runs the challenge and returns the solution
    pub fn run(&mut self, challenge: &Challenge) -> Result<Solution> {
        // Create temp file with starting content
        let temp_file = self
            .filesystem
            .create_temp_file(challenge.starting_content())?;

        // Set up file watching
        let (tx, rx) = mpsc::channel();
        self.watcher.watch(&temp_file, tx)?;

        // Prepare recording if available
        let recording_path = if self.recorder.is_some() {
            use crate::infrastructure::AsciinemaRecorder;
            Some(AsciinemaRecorder::generate_recording_path(challenge.id())?)
        } else {
            None
        };

        // Start timer and spawn editor (with or without recording)
        let start_time = Instant::now();
        let mut recording_process = None;

        if let (Some(recorder), Some(rec_path)) = (self.recorder.as_mut(), &recording_path) {
            // Spawn with recording
            let child = recorder.start_recording(&temp_file, rec_path)?;
            recording_process = Some(child);
        } else {
            // Spawn without recording
            self.editor.spawn(&temp_file)?;
        }

        // Wait for file changes and validate
        let mut completed = false;
        loop {
            // Check if process is still running
            let is_running = if let Some(child) = recording_process.as_mut() {
                child.try_wait()?.is_none()
            } else {
                self.editor.is_running()
            };

            if !is_running {
                break;
            }

            // Check for file change notifications
            if rx.try_recv().is_ok() {
                // Read current content
                let current_content = self.filesystem.read_file(&temp_file)?;

                // Validate against target
                if self
                    .validator
                    .is_valid(&current_content, challenge.target_content())
                {
                    completed = true;
                    // Terminate editor immediately when validation succeeds
                    if let Some(mut child) = recording_process.take() {
                        // Kill asciinema process (which terminates the editor subprocess)
                        let _ = child.kill();
                    } else {
                        self.editor.terminate()?;
                    }
                    break;
                }
            }

            // Small sleep to avoid busy waiting
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let elapsed = start_time.elapsed();

        // Cleanup editor process
        if let Some(mut child) = recording_process {
            let _ = child.wait(); // Wait for asciinema to finish
        } else {
            self.editor.terminate()?;
        }
        self.watcher.stop()?;
        self.filesystem.cleanup(&temp_file)?;

        // Build solution
        let mut solution = if completed {
            Solution::completed(elapsed)
        } else {
            Solution::incomplete(elapsed)
        };

        // Attach recording if available
        if let (Some(recorder), Some(rec_path)) = (self.recorder.as_ref(), recording_path) {
            match recorder.finalize_recording(&rec_path) {
                Ok(recording) => {
                    solution = solution.with_recording(recording);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to finalize recording: {}", e);
                }
            }
        }

        Ok(solution)
    }
}
