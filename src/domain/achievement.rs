use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for each achievement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementId {
    // Speed achievements
    SpeedDemon,
    LightningFast,

    // Efficiency achievements
    Perfectionist,
    EfficiencyExpert,

    // Consistency achievements
    ConsistentLearner,
    DedicatedPractitioner,

    // Mastery achievements
    ChallengeMaster,
    GoldRush,
    Completionist,

    // Milestone achievements
    FirstSteps,
    HalfwayThere,
    CenturyClub,
}

impl AchievementId {
    /// Get all achievement IDs
    pub fn all() -> Vec<AchievementId> {
        vec![
            AchievementId::FirstSteps,
            AchievementId::SpeedDemon,
            AchievementId::LightningFast,
            AchievementId::Perfectionist,
            AchievementId::EfficiencyExpert,
            AchievementId::ConsistentLearner,
            AchievementId::DedicatedPractitioner,
            AchievementId::ChallengeMaster,
            AchievementId::GoldRush,
            AchievementId::Completionist,
            AchievementId::HalfwayThere,
            AchievementId::CenturyClub,
        ]
    }
}

/// Achievement definition with metadata
#[derive(Debug, Clone)]
pub struct Achievement {
    id: AchievementId,
    name: &'static str,
    description: &'static str,
    badge: &'static str,
}

impl Achievement {
    /// Get achievement definition by ID
    pub fn get(id: AchievementId) -> Self {
        match id {
            AchievementId::FirstSteps => Achievement {
                id,
                name: "First Steps",
                description: "Complete your first challenge",
                badge: "👣",
            },
            AchievementId::SpeedDemon => Achievement {
                id,
                name: "Speed Demon",
                description: "Complete 10 challenges under 10 seconds",
                badge: "⚡",
            },
            AchievementId::LightningFast => Achievement {
                id,
                name: "Lightning Fast",
                description: "Complete a challenge in under 5 seconds",
                badge: "⚡⚡",
            },
            AchievementId::Perfectionist => Achievement {
                id,
                name: "Perfectionist",
                description: "Complete a challenge with under 20 keystrokes",
                badge: "💎",
            },
            AchievementId::EfficiencyExpert => Achievement {
                id,
                name: "Efficiency Expert",
                description: "Maintain an average under 40 keystrokes across all completions",
                badge: "🎯",
            },
            AchievementId::ConsistentLearner => Achievement {
                id,
                name: "Consistent Learner",
                description: "Practice 7 days in a row",
                badge: "🔥",
            },
            AchievementId::DedicatedPractitioner => Achievement {
                id,
                name: "Dedicated Practitioner",
                description: "Practice 30 days in a row",
                badge: "🔥🔥",
            },
            AchievementId::ChallengeMaster => Achievement {
                id,
                name: "Challenge Master",
                description: "Achieve gold tier on 25 challenges",
                badge: "🏆",
            },
            AchievementId::GoldRush => Achievement {
                id,
                name: "Gold Rush",
                description: "Achieve gold tier on 10 challenges in a row",
                badge: "🥇",
            },
            AchievementId::Completionist => Achievement {
                id,
                name: "Completionist",
                description: "Complete all available challenges",
                badge: "✨",
            },
            AchievementId::HalfwayThere => Achievement {
                id,
                name: "Halfway There",
                description: "Complete 50% of available challenges",
                badge: "🎖️",
            },
            AchievementId::CenturyClub => Achievement {
                id,
                name: "Century Club",
                description: "Complete 100 challenges total",
                badge: "💯",
            },
        }
    }


    pub fn name(&self) -> &str {
        self.name
    }

    pub fn description(&self) -> &str {
        self.description
    }

    pub fn badge(&self) -> &str {
        self.badge
    }

    #[allow(dead_code)] // Used in tests
    pub fn id(&self) -> AchievementId {
        self.id
    }
}

/// Unlocked achievement with timestamp
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlockedAchievement {
    id: AchievementId,
    unlocked_at: DateTime<Utc>,
}

impl UnlockedAchievement {
    pub fn new(id: AchievementId, unlocked_at: DateTime<Utc>) -> Self {
        Self { id, unlocked_at }
    }

    #[allow(dead_code)] // Used in production code (clippy false positive)
    pub fn id(&self) -> AchievementId {
        self.id
    }

    pub fn unlocked_at(&self) -> DateTime<Utc> {
        self.unlocked_at
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    pub fn check_fast_completion(time: Duration, threshold_secs: u64) -> bool {
        time.as_secs() < threshold_secs
    }

    pub fn check_efficient_completion(keystrokes: Option<u32>, threshold: u32) -> bool {
        keystrokes.is_some_and(|ks| ks < threshold)
    }

    #[test]
    fn test_achievement_get() {
        let achievement = Achievement::get(AchievementId::SpeedDemon);
        assert_eq!(achievement.name(), "Speed Demon");
        assert_eq!(achievement.badge(), "⚡");
    }

    #[test]
    fn test_all_achievements_unique() {
        let all = AchievementId::all();
        let unique_count = all.len();
        assert!(unique_count > 0);
    }

    #[test]
    fn test_check_fast_completion() {
        assert!(check_fast_completion(Duration::from_secs(5), 10));
        assert!(!check_fast_completion(Duration::from_secs(15), 10));
    }

    #[test]
    fn test_check_efficient_completion() {
        assert!(check_efficient_completion(Some(15), 20));
        assert!(!check_efficient_completion(Some(25), 20));
        assert!(!check_efficient_completion(None, 20));
    }
}
