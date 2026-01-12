use crate::domain::{Achievement, AchievementId, MasteryTier, Progress};
use chrono::Utc;

/// Service for checking and unlocking achievements
pub struct AchievementChecker;

impl AchievementChecker {
    /// Check all achievements and return newly unlocked ones
    pub fn check_achievements(
        progress: &mut Progress,
        total_challenges: usize,
    ) -> Vec<Achievement> {
        let mut newly_unlocked = Vec::new();
        let already_unlocked = progress.unlocked_achievement_ids();

        for achievement_id in AchievementId::all() {
            if already_unlocked.contains(&achievement_id) {
                continue; // Already unlocked
            }

            if Self::check_achievement(achievement_id, progress, total_challenges) {
                progress.unlock_achievement(achievement_id, Utc::now());
                newly_unlocked.push(Achievement::get(achievement_id));
            }
        }

        newly_unlocked
    }

    /// Check if a specific achievement should be unlocked
    fn check_achievement(id: AchievementId, progress: &Progress, total_challenges: usize) -> bool {
        match id {
            // First Steps - Complete your first challenge
            AchievementId::FirstSteps => progress.total_completed() >= 1,

            // Speed Demon - Complete 10 challenges under 10 seconds
            AchievementId::SpeedDemon => {
                let fast_completions = progress
                    .all_challenge_stats()
                    .values()
                    .filter(|stats| stats.best_time().map_or(false, |t| t.as_secs() < 10))
                    .count();
                fast_completions >= 10
            }

            // Lightning Fast - Complete a challenge in under 5 seconds
            AchievementId::LightningFast => progress
                .all_challenge_stats()
                .values()
                .any(|stats| stats.best_time().map_or(false, |t| t.as_secs() < 5)),

            // Perfectionist - Complete a challenge with under 20 keystrokes
            AchievementId::Perfectionist => progress
                .all_challenge_stats()
                .values()
                .any(|stats| stats.best_keystrokes().map_or(false, |ks| ks < 20)),

            // Efficiency Expert - Maintain an average under 40 keystrokes
            AchievementId::EfficiencyExpert => {
                progress.average_keystrokes().map_or(false, |avg| avg < 40)
            }

            // Consistent Learner - Practice 7 days in a row
            AchievementId::ConsistentLearner => progress.longest_streak() >= 7,

            // Dedicated Practitioner - Practice 30 days in a row
            AchievementId::DedicatedPractitioner => progress.longest_streak() >= 30,

            // Challenge Master - Achieve gold tier on 25 challenges
            AchievementId::ChallengeMaster => {
                let gold_count = progress
                    .all_challenge_stats()
                    .values()
                    .filter(|stats| {
                        stats
                            .mastery_tier()
                            .map_or(false, |tier| tier == MasteryTier::Gold)
                    })
                    .count();
                gold_count >= 25
            }

            // Gold Rush - Achieve gold tier on 10 challenges in a row
            AchievementId::GoldRush => {
                // Find the longest sequence of consecutive gold completions
                let mut current_streak = 0;
                let mut longest_streak = 0;

                for event in progress.completion_history() {
                    if event.achieved_tier == MasteryTier::Gold {
                        current_streak += 1;
                        if current_streak > longest_streak {
                            longest_streak = current_streak;
                        }
                    } else {
                        // Reset streak for non-gold completion
                        current_streak = 0;
                    }
                }

                longest_streak >= 10
            }

            // Completionist - Complete all available challenges
            AchievementId::Completionist => {
                progress.total_completed() >= total_challenges && total_challenges > 0
            }

            // Halfway There - Complete 50% of available challenges
            AchievementId::HalfwayThere => {
                if total_challenges == 0 {
                    return false;
                }
                let completed = progress.total_completed();
                let halfway = (total_challenges + 1) / 2; // Round up
                completed >= halfway
            }

            // Century Club - Complete 100 challenges total
            AchievementId::CenturyClub => progress.total_completed() >= 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Progress;

    #[test]
    fn test_first_steps_achievement() {
        let mut progress = Progress::new();

        // No achievements initially
        let newly_unlocked = AchievementChecker::check_achievements(&mut progress, 50);
        assert_eq!(newly_unlocked.len(), 0);

        // Record first completion
        progress.record_attempt(
            "test-1".to_string(),
            true,
            std::time::Duration::from_secs(20),
            Some(30),
            Utc::now(),
        );

        // Should unlock FirstSteps
        let newly_unlocked = AchievementChecker::check_achievements(&mut progress, 50);
        assert!(newly_unlocked
            .iter()
            .any(|a| a.id() == AchievementId::FirstSteps));
    }

    #[test]
    fn test_achievement_only_unlocks_once() {
        let mut progress = Progress::new();

        progress.record_attempt(
            "test-1".to_string(),
            true,
            std::time::Duration::from_secs(20),
            Some(30),
            Utc::now(),
        );

        // First check should unlock two achievements: FirstSteps (1 completion) and
        // EfficiencyExpert (average < 40 keystrokes, 30 < 40)
        let newly_unlocked = AchievementChecker::check_achievements(&mut progress, 50);
        assert_eq!(newly_unlocked.len(), 2);
        assert!(newly_unlocked
            .iter()
            .any(|a| a.id() == AchievementId::FirstSteps));
        assert!(newly_unlocked
            .iter()
            .any(|a| a.id() == AchievementId::EfficiencyExpert));

        // Second check should not unlock again
        let newly_unlocked = AchievementChecker::check_achievements(&mut progress, 50);
        assert_eq!(newly_unlocked.len(), 0);
    }

    #[test]
    fn test_speed_demon() {
        let mut progress = Progress::new();

        // Add 10 fast completions
        for i in 0..10 {
            progress.record_attempt(
                format!("test-{}", i),
                true,
                std::time::Duration::from_secs(5),
                Some(20),
                Utc::now(),
            );
        }

        let newly_unlocked = AchievementChecker::check_achievements(&mut progress, 50);
        assert!(newly_unlocked
            .iter()
            .any(|a| a.id() == AchievementId::SpeedDemon));
    }
}
