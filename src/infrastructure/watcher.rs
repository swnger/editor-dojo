use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};

use crate::application::FileWatcher;

/// File watcher implementation using the notify crate
pub struct FileChangeWatcher {
    watcher: Option<RecommendedWatcher>,
}

impl FileChangeWatcher {
    pub fn new() -> Self {
        Self { watcher: None }
    }
}

impl Default for FileChangeWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWatcher for FileChangeWatcher {
    fn watch(&mut self, file_path: &Path, tx: mpsc::Sender<()>) -> Result<()> {
        // Create a debounced watcher to avoid excessive notifications
        let config = Config::default().with_poll_interval(Duration::from_millis(50));

        let mut watcher = RecommendedWatcher::new(
            move |_res| {
                // Send notification on any file event
                let _ = tx.send(());
            },
            config,
        )
        .context("Failed to create file watcher")?;

        watcher
            .watch(file_path, RecursiveMode::NonRecursive)
            .with_context(|| format!("Failed to watch file: {}", file_path.display()))?;

        self.watcher = Some(watcher);
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Dropping the watcher stops it automatically
        self.watcher.take();
        Ok(())
    }
}
