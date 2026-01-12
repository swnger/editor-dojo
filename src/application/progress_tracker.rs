use crate::application::{AchievementChecker, ProgressRepository};
use crate::domain::{Achievement, Progress, Solution};
use anyhow::Result;
use chrono::Utc;
use std::sync::{Arc, Mutex};

/// Application service for tracking and managing user progress
pub struct ProgressTracker<R: ProgressRepository> {
    repository: Arc<R>,
    progress: Arc<Mutex<Progress>>,
}

impl<R: ProgressRepository> ProgressTracker<R> {
    /// Create new progress tracker with given repository
    pub fn new(repository: R) -> Result<Self> {
        let progress = repository.load()?;
        Ok(Self {
            repository: Arc::new(repository),
            progress: Arc::new(Mutex::new(progress)),
        })
    }

    /// Get current progress (thread-safe read)
    pub fn get_progress(&self) -> Progress {
        self.progress.lock().unwrap().clone()
    }

    /// Record a challenge attempt
    pub fn record_solution(&self, challenge_id: &str, solution: &Solution) -> Result<()> {
        let mut progress = self.progress.lock().unwrap();

        let keystrokes = solution
            .recording()
            .map(|r| r.keystroke_count() as u32);

        progress.record_attempt(
            challenge_id.to_string(),
            solution.is_completed(),
            solution.elapsed_time(),
            keystrokes,
            Utc::now(),
        );

        self.repository.save(&progress)?;
        Ok(())
    }

    /// Set editor preference
    pub fn set_editor_preference(&self, editor: String) -> Result<()> {
        let mut progress = self.progress.lock().unwrap();
        *progress = progress.clone().set_editor_preference(editor);
        self.repository.save(&progress)?;
        Ok(())
    }

    /// Check for new achievements and update progress
    pub fn check_achievements(&self, total_challenges: usize) -> Result<Vec<Achievement>> {
        let mut progress = self.progress.lock().unwrap();
        let newly_unlocked = AchievementChecker::check_achievements(&mut progress, total_challenges);

        if !newly_unlocked.is_empty() {
            self.repository.save(&progress)?;
        }

        Ok(newly_unlocked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Solution;
    use std::time::Duration;

    // Mock repository for testing
    struct MockRepository {
        progress: Mutex<Progress>,
    }

    impl MockRepository {
        fn new() -> Self {
            Self {
                progress: Mutex::new(Progress::new()),
            }
        }
    }

    impl ProgressRepository for MockRepository {
        fn load(&self) -> Result<Progress> {
            Ok(self.progress.lock().unwrap().clone())
        }

        fn save(&self, progress: &Progress) -> Result<()> {
            *self.progress.lock().unwrap() = progress.clone();
            Ok(())
        }
    }

    #[test]
    fn test_record_completed_solution() {
        let repo = MockRepository::new();
        let tracker = ProgressTracker::new(repo).unwrap();

        let solution = Solution::completed(Duration::from_secs(10));
        tracker.record_solution("test-1", &solution).unwrap();

        let progress = tracker.get_progress();
        assert_eq!(progress.total_completed(), 1);
        assert_eq!(progress.total_practice_time(), Duration::from_secs(10));
    }
}
