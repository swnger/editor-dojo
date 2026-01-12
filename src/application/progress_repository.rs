use crate::domain::Progress;
use anyhow::Result;

/// Repository interface for persisting and loading progress
pub trait ProgressRepository {
    /// Load progress from storage
    /// Returns a new Progress if none exists
    fn load(&self) -> Result<Progress>;

    /// Save progress to storage
    fn save(&self, progress: &Progress) -> Result<()>;
}
