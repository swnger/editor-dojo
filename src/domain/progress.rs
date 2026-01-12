use crate::domain::challenge_stats::ChallengeStats;
use crate::domain::achievement::{AchievementId, UnlockedAchievement};
use crate::domain::MasteryTier;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Represents a completion event in the history
#[derive(Debug, Clone)]
pub struct CompletionEvent {
    pub challenge_id: String,
    pub achieved_tier: MasteryTier,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionEventDto {
    challenge_id: String,
    achieved_tier: MasteryTier,
    completed_at: DateTime<Utc>,
}

/// Entity representing user's overall progress
#[derive(Debug, Clone)]
pub struct Progress {
    challenge_stats: HashMap<String, ChallengeStats>,
    total_practice_time: Duration,
    last_practice_date: Option<NaiveDate>,
    longest_streak: u32,
    editor_preference: Option<String>,
    unlocked_achievements: HashMap<AchievementId, UnlockedAchievement>,
    /// Track completion history for sequential achievements (e.g., GoldRush)
    completion_history: Vec<CompletionEvent>,
}

impl Progress {
    /// Create new empty progress
    pub fn new() -> Self {
        Self {
            challenge_stats: HashMap::new(),
            total_practice_time: Duration::ZERO,
            last_practice_date: None,
            longest_streak: 0,
            editor_preference: None,
            unlocked_achievements: HashMap::new(),
            completion_history: Vec::new(),
        }
    }

    /// Create progress with initial values
    pub fn with_values(
        challenge_stats: HashMap<String, ChallengeStats>,
        total_practice_time: Duration,
        last_practice_date: Option<NaiveDate>,
        longest_streak: u32,
        editor_preference: Option<String>,
        unlocked_achievements: HashMap<AchievementId, UnlockedAchievement>,
    ) -> Self {
        Self {
            challenge_stats,
            total_practice_time,
            last_practice_date,
            longest_streak,
            editor_preference,
            unlocked_achievements,
            completion_history: Vec::new(),
        }
    }

    /// Set editor preference
    pub fn set_editor_preference(mut self, editor: String) -> Self {
        self.editor_preference = Some(editor);
        self
    }

    /// Set completion history (for deserialization)
    pub fn with_completion_history(mut self, history: Vec<CompletionEvent>) -> Self {
        self.completion_history = history;
        self
    }

    /// Get stats for a specific challenge
    pub fn get_challenge_stats(&self, challenge_id: &str) -> Option<&ChallengeStats> {
        self.challenge_stats.get(challenge_id)
    }

    /// Update stats for a challenge after an attempt
    pub fn record_attempt(
        &mut self,
        challenge_id: String,
        completed: bool,
        time: Duration,
        keystrokes: Option<u32>,
        attempted_at: DateTime<Utc>,
    ) {
        let attempted_date = attempted_at.date_naive();

        // Update or create challenge stats
        let updated_stats = if let Some(existing) = self.challenge_stats.get(&challenge_id) {
            existing.record_attempt(completed, time, keystrokes, attempted_at)
        } else {
            if completed {
                ChallengeStats::completed(challenge_id.clone(), time, keystrokes, attempted_at)
            } else {
                let stats = ChallengeStats::new(challenge_id.clone());
                stats.record_attempt(completed, time, keystrokes, attempted_at)
            }
        };

        self.challenge_stats.insert(challenge_id.clone(), updated_stats);

        // Update total practice time
        self.total_practice_time += time;

        // Update last practice date
        self.last_practice_date = Some(attempted_date);

        // Update streak if this is a completion
        if completed {
            let current_streak = self.calculate_current_streak(attempted_date);
            if current_streak > self.longest_streak {
                self.longest_streak = current_streak;
            }

            // Add to completion history for sequential achievements
            let current_stats = self.challenge_stats.get(&challenge_id).unwrap();
            if let Some(tier) = current_stats.mastery_tier() {
                self.completion_history.push(CompletionEvent {
                    challenge_id: challenge_id.clone(),
                    achieved_tier: tier,
                    completed_at: attempted_at,
                });
            }
        }
    }

    /// Calculate current streak based on last practice date
    pub fn calculate_current_streak(&self, today: NaiveDate) -> u32 {
        let last_practice = match self.last_practice_date {
            Some(date) => date,
            None => return 0,
        };

        let days_since = (today - last_practice).num_days();

        // If more than 1 day has passed, streak is broken
        if days_since > 1 {
            return 0;
        }

        // Count consecutive days with completions
        let mut streak = 0;
        let mut check_date = today;

        loop {
            let has_completion = self.has_completion_on_date(check_date);
            if !has_completion {
                break;
            }
            streak += 1;
            // pred_opt() returns None only for dates before year 1, which won't happen in practice
            check_date = check_date.pred_opt().expect("Date pred_opt should not return None");
        }

        streak
    }

    /// Check if there's at least one completion on a specific date
    fn has_completion_on_date(&self, date: NaiveDate) -> bool {
        self.challenge_stats.values().any(|stats| {
            if let Some(first_completed) = stats.first_completed_at() {
                first_completed.date_naive() == date
            } else {
                false
            }
        })
    }

    /// Get total number of completed challenges
    pub fn total_completed(&self) -> usize {
        self.challenge_stats
            .values()
            .filter(|stats| stats.is_completed())
            .count()
    }

    /// Get all challenge stats as a map
    pub fn all_challenge_stats(&self) -> &HashMap<String, ChallengeStats> {
        &self.challenge_stats
    }

    /// Get total practice time
    pub fn total_practice_time(&self) -> Duration {
        self.total_practice_time
    }

    /// Get last practice date
    pub fn last_practice_date(&self) -> Option<NaiveDate> {
        self.last_practice_date
    }

    /// Get longest streak
    pub fn longest_streak(&self) -> u32 {
        self.longest_streak
    }

    /// Get editor preference
    pub fn editor_preference(&self) -> Option<&str> {
        self.editor_preference.as_deref()
    }

    /// Get total attempts across all challenges
    pub fn total_attempts(&self) -> u32 {
        self.challenge_stats
            .values()
            .map(|stats| stats.attempt_count())
            .sum()
    }

    /// Get average solve time for completed challenges
    pub fn average_solve_time(&self) -> Option<Duration> {
        let completed: Vec<_> = self
            .challenge_stats
            .values()
            .filter_map(|stats| stats.best_time())
            .collect();

        if completed.is_empty() {
            return None;
        }

        let total_secs: u64 = completed.iter().map(|d| d.as_secs()).sum();
        let avg_secs = total_secs / completed.len() as u64;
        Some(Duration::from_secs(avg_secs))
    }

    /// Get average keystrokes for completed challenges
    pub fn average_keystrokes(&self) -> Option<u32> {
        let keystrokes: Vec<_> = self
            .challenge_stats
            .values()
            .filter_map(|stats| stats.best_keystrokes())
            .collect();

        if keystrokes.is_empty() {
            return None;
        }

        let total: u32 = keystrokes.iter().sum();
        Some(total / keystrokes.len() as u32)
    }

    /// Get recently completed challenges (sorted by completion date, most recent first)
    pub fn recently_completed(&self, limit: usize) -> Vec<&ChallengeStats> {
        let mut completed: Vec<_> = self
            .challenge_stats
            .values()
            .filter(|stats| stats.is_completed())
            .collect();

        completed.sort_by(|a, b| {
            b.last_attempted_at()
                .cmp(&a.last_attempted_at())
        });

        completed.into_iter().take(limit).collect()
    }

    /// Unlock an achievement
    pub fn unlock_achievement(&mut self, id: AchievementId, unlocked_at: DateTime<Utc>) {
        if !self.unlocked_achievements.contains_key(&id) {
            self.unlocked_achievements.insert(id, UnlockedAchievement::new(id, unlocked_at));
        }
    }

    /// Check if an achievement is unlocked
    pub fn is_achievement_unlocked(&self, id: AchievementId) -> bool {
        self.unlocked_achievements.contains_key(&id)
    }

    /// Get all unlocked achievements
    pub fn unlocked_achievements(&self) -> Vec<&UnlockedAchievement> {
        let mut achievements: Vec<_> = self.unlocked_achievements.values().collect();
        achievements.sort_by_key(|a| a.unlocked_at());
        achievements
    }

    /// Get count of unlocked achievements
    pub fn achievement_count(&self) -> usize {
        self.unlocked_achievements.len()
    }

    /// Get all unlocked achievement IDs as a set
    pub fn unlocked_achievement_ids(&self) -> HashSet<AchievementId> {
        self.unlocked_achievements.keys().copied().collect()
    }

    /// Get completion history for sequential achievements
    pub fn completion_history(&self) -> &[CompletionEvent] {
        &self.completion_history
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_new_progress_is_empty() {
        let progress = Progress::new();
        assert_eq!(progress.total_completed(), 0);
        assert_eq!(progress.total_practice_time(), Duration::ZERO);
        assert_eq!(progress.longest_streak(), 0);
        assert!(progress.last_practice_date().is_none());
    }

    #[test]
    fn test_record_first_completion() {
        let mut progress = Progress::new();
        let now = Utc::now();

        progress.record_attempt(
            "test-1".to_string(),
            true,
            Duration::from_secs(10),
            Some(15),
            now,
        );

        assert_eq!(progress.total_completed(), 1);
        assert_eq!(progress.total_practice_time(), Duration::from_secs(10));
        assert_eq!(progress.last_practice_date(), Some(now.date_naive()));
    }

    #[test]
    fn test_record_multiple_attempts_same_challenge() {
        let mut progress = Progress::new();
        let now = Utc::now();

        progress.record_attempt(
            "test-1".to_string(),
            true,
            Duration::from_secs(10),
            Some(15),
            now,
        );

        let later = now + chrono::Duration::seconds(60);
        progress.record_attempt(
            "test-1".to_string(),
            true,
            Duration::from_secs(8),
            Some(12),
            later,
        );

        assert_eq!(progress.total_completed(), 1);
        assert_eq!(
            progress.total_practice_time(),
            Duration::from_secs(18)
        );

        let stats = progress.get_challenge_stats("test-1").unwrap();
        assert_eq!(stats.attempt_count(), 2);
        assert_eq!(stats.best_time(), Some(Duration::from_secs(8)));
    }

    #[test]
    fn test_streak_calculation() {
        let mut progress = Progress::new();

        // Day 1
        let day1 = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        progress.record_attempt(
            "test-1".to_string(),
            true,
            Duration::from_secs(10),
            Some(15),
            day1,
        );

        let streak = progress.calculate_current_streak(day1.date_naive());
        assert_eq!(streak, 1);

        // Day 2
        let day2 = Utc.with_ymd_and_hms(2025, 1, 2, 12, 0, 0).unwrap();
        progress.record_attempt(
            "test-2".to_string(),
            true,
            Duration::from_secs(10),
            Some(15),
            day2,
        );

        let streak = progress.calculate_current_streak(day2.date_naive());
        assert_eq!(streak, 2);
        assert_eq!(progress.longest_streak(), 2);
    }

    #[test]
    fn test_streak_breaks_after_missing_day() {
        let mut progress = Progress::new();

        // Day 1
        let day1 = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        progress.record_attempt(
            "test-1".to_string(),
            true,
            Duration::from_secs(10),
            Some(15),
            day1,
        );

        // Skip day 2, practice on day 3
        let day3 = Utc.with_ymd_and_hms(2025, 1, 3, 12, 0, 0).unwrap();
        let streak = progress.calculate_current_streak(day3.date_naive());
        assert_eq!(streak, 0); // Streak broken

        // But longest streak is preserved
        assert_eq!(progress.longest_streak(), 1);
    }

    #[test]
    fn test_average_calculations() {
        let mut progress = Progress::new();
        let now = Utc::now();

        progress.record_attempt(
            "test-1".to_string(),
            true,
            Duration::from_secs(10),
            Some(15),
            now,
        );

        progress.record_attempt(
            "test-2".to_string(),
            true,
            Duration::from_secs(20),
            Some(25),
            now,
        );

        assert_eq!(progress.average_solve_time(), Some(Duration::from_secs(15)));
        assert_eq!(progress.average_keystrokes(), Some(20));
    }
}
