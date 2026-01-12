use crate::application::ProgressRepository;
use crate::domain::{AchievementId, ChallengeStats, Progress, UnlockedAchievement};
use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// JSON-based progress repository implementation
pub struct JsonProgressRepository {
    file_path: PathBuf,
}

impl JsonProgressRepository {
    /// Create new repository with default path
    pub fn new() -> Result<Self> {
        let file_path = Self::default_progress_path()?;
        Ok(Self { file_path })
    }

    /// Create repository with custom path (useful for testing)
    pub fn with_path(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    /// Get default progress file path based on OS
    fn default_progress_path() -> Result<PathBuf> {
        let data_dir = if cfg!(target_os = "windows") {
            // Windows: %APPDATA%/editor-dojo
            dirs::data_dir()
                .context("Failed to get APPDATA directory")?
                .join("editor-dojo")
        } else {
            // Linux/Mac: ~/.local/share/editor-dojo
            dirs::data_local_dir()
                .context("Failed to get local data directory")?
                .join("editor-dojo")
        };

        // Create directory if it doesn't exist
        fs::create_dir_all(&data_dir)?;

        Ok(data_dir.join("progress.json"))
    }

    /// Backup corrupted progress file
    fn backup_corrupted_file(&self) -> Result<()> {
        if self.file_path.exists() {
            let backup_path = self.file_path.with_extension("json.bak");
            fs::copy(&self.file_path, &backup_path)?;
        }
        Ok(())
    }
}

impl Default for JsonProgressRepository {
    fn default() -> Self {
        Self::new().expect("Failed to create JsonProgressRepository")
    }
}

impl ProgressRepository for JsonProgressRepository {
    fn load(&self) -> Result<Progress> {
        if !self.file_path.exists() {
            return Ok(Progress::new());
        }

        let json = fs::read_to_string(&self.file_path).context(format!(
            "Failed to read progress file: {}",
            self.file_path.display()
        ))?;

        let dto: ProgressDto = match serde_json::from_str(&json) {
            Ok(dto) => dto,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse progress file: {}. Creating backup and starting fresh.",
                    e
                );
                self.backup_corrupted_file()?;
                ProgressDto::default()
            }
        };

        Ok(dto.to_domain())
    }

    fn save(&self, progress: &Progress) -> Result<()> {
        let dto = ProgressDto::from_domain(progress);
        let json = serde_json::to_string_pretty(&dto)?;

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.file_path, json).context(format!(
            "Failed to write progress file: {}",
            self.file_path.display()
        ))?;

        Ok(())
    }

    fn exists(&self) -> bool {
        self.file_path.exists()
    }
}

// Data Transfer Objects for JSON serialization
#[derive(Debug, Serialize, Deserialize, Default)]
struct ProgressDto {
    editor_preference: Option<String>,
    total_practice_time_secs: u64,
    last_practice_date: Option<String>,
    longest_streak: u32,
    challenges: HashMap<String, ChallengeStatsDto>,
    #[serde(default)]
    unlocked_achievements: Vec<UnlockedAchievementDto>,
    #[serde(default)]
    completion_history: Vec<CompletionEventDto>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompletionEventDto {
    challenge_id: String,
    achieved_tier: crate::domain::MasteryTier,
    completed_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChallengeStatsDto {
    completed: bool,
    best_time_secs: Option<u64>,
    best_keystrokes: Option<u32>,
    first_completed_at: Option<String>,
    last_attempted_at: Option<String>,
    attempt_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct UnlockedAchievementDto {
    id: AchievementId,
    unlocked_at: String,
}

impl ProgressDto {
    fn from_domain(progress: &Progress) -> Self {
        let challenges = progress
            .all_challenge_stats()
            .iter()
            .map(|(id, stats)| (id.clone(), ChallengeStatsDto::from_domain(stats)))
            .collect();

        let unlocked_achievements = progress
            .unlocked_achievements()
            .iter()
            .map(|a| UnlockedAchievementDto::from_domain(a))
            .collect();

        let completion_history = progress
            .completion_history()
            .iter()
            .map(|event| CompletionEventDto::from_domain(event))
            .collect();

        Self {
            editor_preference: progress.editor_preference().map(|s| s.to_string()),
            total_practice_time_secs: progress.total_practice_time().as_secs(),
            last_practice_date: progress.last_practice_date().map(|d| d.to_string()),
            longest_streak: progress.longest_streak(),
            challenges,
            unlocked_achievements,
            completion_history,
        }
    }

    fn to_domain(self) -> Progress {
        let challenge_stats = self
            .challenges
            .into_iter()
            .map(|(id, dto)| (id.clone(), dto.to_domain(id)))
            .collect();

        let last_practice_date = self
            .last_practice_date
            .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

        let unlocked_achievements = self
            .unlocked_achievements
            .into_iter()
            .filter_map(|dto| dto.to_domain())
            .map(|a| (a.id(), a))
            .collect();

        let completion_history = self
            .completion_history
            .into_iter()
            .filter_map(|dto| dto.to_domain())
            .collect();

        Progress::with_values(
            challenge_stats,
            Duration::from_secs(self.total_practice_time_secs),
            last_practice_date,
            self.longest_streak,
            self.editor_preference,
            unlocked_achievements,
        )
        .with_completion_history(completion_history)
    }
}

impl CompletionEventDto {
    fn from_domain(event: &crate::domain::progress::CompletionEvent) -> Self {
        Self {
            challenge_id: event.challenge_id.clone(),
            achieved_tier: event.achieved_tier,
            completed_at: event.completed_at.to_rfc3339(),
        }
    }

    fn to_domain(self) -> Option<crate::domain::progress::CompletionEvent> {
        use crate::domain::progress::CompletionEvent;

        DateTime::parse_from_rfc3339(&self.completed_at)
            .ok()
            .map(|dt| CompletionEvent {
                challenge_id: self.challenge_id,
                achieved_tier: self.achieved_tier,
                completed_at: dt.with_timezone(&Utc),
            })
    }
}

impl ChallengeStatsDto {
    fn from_domain(stats: &ChallengeStats) -> Self {
        Self {
            completed: stats.is_completed(),
            best_time_secs: stats.best_time().map(|d| d.as_secs()),
            best_keystrokes: stats.best_keystrokes(),
            first_completed_at: stats
                .first_completed_at()
                .map(|dt| dt.to_rfc3339()),
            last_attempted_at: stats
                .last_attempted_at()
                .map(|dt| dt.to_rfc3339()),
            attempt_count: stats.attempt_count(),
        }
    }

    fn to_domain(self, challenge_id: String) -> ChallengeStats {
        let first_completed_at = self
            .first_completed_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let last_attempted_at = self
            .last_attempted_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        // Reconstruct ChallengeStats using public methods
        let mut stats = ChallengeStats::new(challenge_id);

        if self.completed && self.best_time_secs.is_some() {
            let time = Duration::from_secs(self.best_time_secs.unwrap());
            let completed_at = first_completed_at.unwrap_or_else(Utc::now);

            // Create initial completed stats
            stats = ChallengeStats::completed(
                stats.challenge_id().to_string(),
                time,
                self.best_keystrokes,
                completed_at,
            );

            // Record additional attempts if needed
            for _ in 1..self.attempt_count {
                let attempt_at = last_attempted_at.unwrap_or(completed_at);
                stats = stats.record_attempt(
                    true,
                    time,
                    self.best_keystrokes,
                    attempt_at,
                );
            }
        }

        stats
    }
}

impl UnlockedAchievementDto {
    fn from_domain(achievement: &UnlockedAchievement) -> Self {
        Self {
            id: achievement.id(),
            unlocked_at: achievement.unlocked_at().to_rfc3339(),
        }
    }

    fn to_domain(self) -> Option<UnlockedAchievement> {
        DateTime::parse_from_rfc3339(&self.unlocked_at)
            .ok()
            .map(|dt| UnlockedAchievement::new(self.id, dt.with_timezone(&Utc)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_progress() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("progress.json");
        let repo = JsonProgressRepository::with_path(file_path);

        let mut progress = Progress::new();
        progress.record_attempt(
            "test-1".to_string(),
            true,
            Duration::from_secs(10),
            Some(15),
            Utc::now(),
        );

        repo.save(&progress).unwrap();
        let loaded = repo.load().unwrap();

        assert_eq!(loaded.total_completed(), 1);
        assert_eq!(loaded.total_practice_time(), Duration::from_secs(10));
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.json");
        let repo = JsonProgressRepository::with_path(file_path);

        let progress = repo.load().unwrap();
        assert_eq!(progress.total_completed(), 0);
    }

    #[test]
    fn test_corrupted_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("progress.json");

        // Write corrupted JSON
        fs::write(&file_path, "{ invalid json }").unwrap();

        let repo = JsonProgressRepository::with_path(file_path.clone());
        let progress = repo.load().unwrap();

        // Should return empty progress and create backup
        assert_eq!(progress.total_completed(), 0);
        assert!(file_path.with_extension("json.bak").exists());
    }
}
