use chrono::{DateTime, Utc};
use std::time::Duration;
use crate::domain::MasteryTier;

/// Value object representing statistics for a single challenge
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeStats {
    challenge_id: String,
    completed: bool,
    best_time: Option<Duration>,
    best_keystrokes: Option<u32>,
    first_completed_at: Option<DateTime<Utc>>,
    last_attempted_at: Option<DateTime<Utc>>,
    attempt_count: u32,
}

impl ChallengeStats {
    /// Create new stats for an unattempted challenge
    pub fn new(challenge_id: String) -> Self {
        Self {
            challenge_id,
            completed: false,
            best_time: None,
            best_keystrokes: None,
            first_completed_at: None,
            last_attempted_at: None,
            attempt_count: 0,
        }
    }

    /// Create stats for a completed challenge
    pub fn completed(
        challenge_id: String,
        time: Duration,
        keystrokes: Option<u32>,
        completed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            challenge_id,
            completed: true,
            best_time: Some(time),
            best_keystrokes: keystrokes,
            first_completed_at: Some(completed_at),
            last_attempted_at: Some(completed_at),
            attempt_count: 1,
        }
    }

    /// Update stats with a new attempt
    pub fn record_attempt(
        &self,
        completed: bool,
        time: Duration,
        keystrokes: Option<u32>,
        attempted_at: DateTime<Utc>,
    ) -> Self {
        let mut updated = self.clone();
        updated.attempt_count += 1;
        updated.last_attempted_at = Some(attempted_at);

        if completed {
            updated.completed = true;

            if updated.first_completed_at.is_none() {
                updated.first_completed_at = Some(attempted_at);
            }

            // Update best time if this is better
            let is_better_time = updated.best_time.is_none_or(|best| time < best);
            if is_better_time {
                updated.best_time = Some(time);
            }

            // Update best keystrokes if this is better
            if let Some(new_keystrokes) = keystrokes {
                let is_better_keystrokes = updated.best_keystrokes.is_none_or(|best| new_keystrokes < best);
                if is_better_keystrokes {
                    updated.best_keystrokes = Some(new_keystrokes);
                }
            }
        }

        updated
    }


    /// Check if this attempt beats any personal record
    #[allow(dead_code)] // Used in tests
    pub fn is_new_record(&self, time: Duration, keystrokes: Option<u32>) -> (bool, bool) {
        let new_time_record = self.best_time.is_none_or(|best| time < best);
        let new_keystroke_record = keystrokes.is_some_and(|new_ks| {
            self.best_keystrokes.is_none_or(|best| new_ks < best)
        });
        (new_time_record, new_keystroke_record)
    }

    // Getters
    pub fn challenge_id(&self) -> &str {
        &self.challenge_id
    }

    pub fn is_completed(&self) -> bool {
        self.completed
    }

    pub fn best_time(&self) -> Option<Duration> {
        self.best_time
    }

    pub fn best_keystrokes(&self) -> Option<u32> {
        self.best_keystrokes
    }

    pub fn first_completed_at(&self) -> Option<DateTime<Utc>> {
        self.first_completed_at
    }

    pub fn last_attempted_at(&self) -> Option<DateTime<Utc>> {
        self.last_attempted_at
    }

    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
    }

    /// Get mastery tier for this challenge based on best performance
    pub fn mastery_tier(&self) -> Option<MasteryTier> {
        if !self.completed {
            return None;
        }

        self.best_time.map(|time| MasteryTier::calculate(time, self.best_keystrokes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_new_stats_starts_unattempted() {
        let stats = ChallengeStats::new("test-1".to_string());
        assert_eq!(stats.challenge_id(), "test-1");
        assert!(!stats.is_completed());
        assert_eq!(stats.attempt_count(), 0);
        assert!(stats.best_time().is_none());
        assert!(stats.best_keystrokes().is_none());
    }

    #[test]
    fn test_completed_stats() {
        let now = Utc::now();
        let stats = ChallengeStats::completed(
            "test-1".to_string(),
            Duration::from_secs(10),
            Some(15),
            now,
        );

        assert!(stats.is_completed());
        assert_eq!(stats.attempt_count(), 1);
        assert_eq!(stats.best_time(), Some(Duration::from_secs(10)));
        assert_eq!(stats.best_keystrokes(), Some(15));
        assert_eq!(stats.first_completed_at(), Some(now));
    }

    #[test]
    fn test_record_better_attempt() {
        let now = Utc::now();
        let stats = ChallengeStats::completed(
            "test-1".to_string(),
            Duration::from_secs(10),
            Some(15),
            now,
        );

        let later = now + chrono::Duration::seconds(60);
        let updated = stats.record_attempt(
            true,
            Duration::from_secs(8),
            Some(12),
            later,
        );

        assert_eq!(updated.attempt_count(), 2);
        assert_eq!(updated.best_time(), Some(Duration::from_secs(8)));
        assert_eq!(updated.best_keystrokes(), Some(12));
        assert_eq!(updated.first_completed_at(), Some(now)); // Unchanged
        assert_eq!(updated.last_attempted_at(), Some(later));
    }

    #[test]
    fn test_record_worse_attempt() {
        let now = Utc::now();
        let stats = ChallengeStats::completed(
            "test-1".to_string(),
            Duration::from_secs(10),
            Some(15),
            now,
        );

        let later = now + chrono::Duration::seconds(60);
        let updated = stats.record_attempt(
            true,
            Duration::from_secs(12),
            Some(18),
            later,
        );

        assert_eq!(updated.attempt_count(), 2);
        assert_eq!(updated.best_time(), Some(Duration::from_secs(10))); // Unchanged
        assert_eq!(updated.best_keystrokes(), Some(15)); // Unchanged
    }

    #[test]
    fn test_is_new_record() {
        let now = Utc::now();
        let stats = ChallengeStats::completed(
            "test-1".to_string(),
            Duration::from_secs(10),
            Some(15),
            now,
        );

        let (new_time, new_ks) = stats.is_new_record(Duration::from_secs(8), Some(12));
        assert!(new_time);
        assert!(new_ks);

        let (new_time, new_ks) = stats.is_new_record(Duration::from_secs(12), Some(18));
        assert!(!new_time);
        assert!(!new_ks);

        let (new_time, new_ks) = stats.is_new_record(Duration::from_secs(8), Some(18));
        assert!(new_time);
        assert!(!new_ks);
    }
}
