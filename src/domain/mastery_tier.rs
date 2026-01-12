use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Mastery tier for a challenge based on performance
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MasteryTier {
    Bronze,
    Silver,
    Gold,
}

impl MasteryTier {
    /// Calculate mastery tier based on time and keystrokes
    ///
    /// Tiers:
    /// - 🥉 Bronze: Complete the challenge
    /// - 🥈 Silver: Complete under 30s and 50 keystrokes
    /// - 🥇 Gold: Complete under 15s and 30 keystrokes
    pub fn calculate(time: Duration, keystrokes: Option<u32>) -> Self {
        let time_secs = time.as_secs();

        // Gold tier requirements: under 15s AND under 30 keystrokes
        if time_secs < 15 {
            if let Some(ks) = keystrokes {
                if ks < 30 {
                    return MasteryTier::Gold;
                }
            }
        }

        // Silver tier requirements: under 30s AND under 50 keystrokes
        if time_secs < 30 {
            if let Some(ks) = keystrokes {
                if ks < 50 {
                    return MasteryTier::Silver;
                }
            }
        }

        // Bronze tier: just completed
        MasteryTier::Bronze
    }

    /// Get the display name of the tier
    pub fn name(&self) -> &str {
        match self {
            MasteryTier::Bronze => "Bronze",
            MasteryTier::Silver => "Silver",
            MasteryTier::Gold => "Gold",
        }
    }

    /// Get the emoji representation of the tier
    pub fn emoji(&self) -> &str {
        match self {
            MasteryTier::Bronze => "🥉",
            MasteryTier::Silver => "🥈",
            MasteryTier::Gold => "🥇",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gold_tier() {
        let tier = MasteryTier::calculate(Duration::from_secs(10), Some(25));
        assert_eq!(tier, MasteryTier::Gold);
    }

    #[test]
    fn test_silver_tier() {
        let tier = MasteryTier::calculate(Duration::from_secs(20), Some(40));
        assert_eq!(tier, MasteryTier::Silver);
    }

    #[test]
    fn test_bronze_tier() {
        let tier = MasteryTier::calculate(Duration::from_secs(60), Some(100));
        assert_eq!(tier, MasteryTier::Bronze);
    }

    #[test]
    fn test_bronze_no_keystrokes() {
        let tier = MasteryTier::calculate(Duration::from_secs(10), None);
        assert_eq!(tier, MasteryTier::Bronze);
    }

    #[test]
    fn test_tier_ordering() {
        assert!(MasteryTier::Gold > MasteryTier::Silver);
        assert!(MasteryTier::Silver > MasteryTier::Bronze);
    }
}
