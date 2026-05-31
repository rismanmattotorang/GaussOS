// src/memory/decay.rs
//! Forgetting, reinforcement, and salience scoring for GaussOS.
//!
//! Unbounded memory growth degrades both retrieval quality and cost. Biological
//! memory solves this with a *forgetting curve* (Ebbinghaus): retention decays
//! exponentially with time but is reinforced by repeated access. GaussOS uses
//! the same principle to decide which memories stay hot, which get archived, and
//! which can be safely forgotten — the kind of housekeeping Letta performs during
//! "sleep-time" idle periods.
//!
//! The retention score combines four signals:
//! 1. **Recency** — exponential decay since last access.
//! 2. **Frequency** — log-scaled access count (spacing effect).
//! 3. **Importance** — intrinsic quality/priority of the memory.
//! 4. **Reinforcement** — repeated retrieval flattens the decay curve.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::{MemCube, Priority};

/// Tunable parameters for the forgetting model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Base retention half-life in seconds (default: 30 days).
    pub base_half_life_secs: f64,
    /// How much each additional access extends the half-life (multiplicative).
    pub reinforcement_factor: f64,
    /// Weight of the recency component in `[0.0, 1.0]`.
    pub recency_weight: f64,
    /// Weight of the frequency component.
    pub frequency_weight: f64,
    /// Weight of the importance component.
    pub importance_weight: f64,
    /// Below this retention score a memory is a candidate for archival.
    pub archive_threshold: f64,
    /// Below this retention score a memory is a candidate for forgetting.
    pub forget_threshold: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            base_half_life_secs: 30.0 * 24.0 * 3600.0,
            reinforcement_factor: 0.35,
            recency_weight: 0.5,
            frequency_weight: 0.3,
            importance_weight: 0.2,
            archive_threshold: 0.25,
            forget_threshold: 0.05,
        }
    }
}

/// Recommended retention action for a memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionAction {
    /// Keep in active/hot storage.
    Retain,
    /// Move to cold storage; still retrievable but de-prioritised.
    Archive,
    /// Eligible to be forgotten (deleted or consolidated away).
    Forget,
}

/// A breakdown of the retention computation for a single memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionScore {
    pub recency: f64,
    pub frequency: f64,
    pub importance: f64,
    /// Combined retention strength in `[0.0, 1.0]`.
    pub score: f64,
    pub action: RetentionAction,
}

/// Computes forgetting-curve retention scores.
#[derive(Debug, Clone)]
pub struct ForgettingCurve {
    config: DecayConfig,
}

impl Default for ForgettingCurve {
    fn default() -> Self {
        Self::new(DecayConfig::default())
    }
}

impl ForgettingCurve {
    pub fn new(config: DecayConfig) -> Self {
        Self { config }
    }

    /// Effective half-life after reinforcement from repeated access. Each access
    /// extends durability following a diminishing (log) curve.
    fn effective_half_life(&self, access_count: u64) -> f64 {
        let reinforcement = 1.0 + self.config.reinforcement_factor * (access_count as f64).ln_1p();
        self.config.base_half_life_secs * reinforcement
    }

    /// Map a [`Priority`] onto an importance value in `[0.0, 1.0]`.
    fn priority_importance(priority: &Priority) -> f64 {
        match priority {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.6,
            Priority::Normal => 0.4,
            Priority::Low => 0.2,
            Priority::Archive => 0.05,
        }
    }

    /// Compute the retention score for a memory as of `now`.
    pub fn retention(&self, cube: &MemCube, now: DateTime<Utc>) -> RetentionScore {
        let age = (now - cube.metadata.last_accessed).num_seconds().max(0) as f64;
        let half_life = self.effective_half_life(cube.metadata.access_count);
        let recency = 0.5f64.powf(age / half_life);

        // Spacing effect: frequency contributes on a saturating log scale.
        let frequency = ((cube.metadata.access_count as f64).ln_1p() / 6.0).min(1.0);

        // Importance blends intrinsic quality with declared priority.
        let importance = (cube.metadata.quality_score.clamp(0.0, 1.0)
            + Self::priority_importance(&cube.metadata.priority))
            / 2.0;

        let total_weight = self.config.recency_weight
            + self.config.frequency_weight
            + self.config.importance_weight;
        let score = (self.config.recency_weight * recency
            + self.config.frequency_weight * frequency
            + self.config.importance_weight * importance)
            / total_weight;

        let action = if score < self.config.forget_threshold {
            RetentionAction::Forget
        } else if score < self.config.archive_threshold {
            RetentionAction::Archive
        } else {
            RetentionAction::Retain
        };

        RetentionScore {
            recency,
            frequency,
            importance,
            score,
            action,
        }
    }

    /// Partition a set of memories into (retain, archive, forget) buckets — the
    /// core of a sleep-time consolidation pass.
    pub fn classify<'a>(
        &self,
        cubes: impl IntoIterator<Item = &'a MemCube>,
    ) -> RetentionPlan {
        let now = Utc::now();
        let mut plan = RetentionPlan::default();
        for cube in cubes {
            // Pinned/critical memories are never forgotten.
            if matches!(cube.metadata.priority, Priority::Critical) {
                plan.retain.push(cube.id);
                continue;
            }
            match self.retention(cube, now).action {
                RetentionAction::Retain => plan.retain.push(cube.id),
                RetentionAction::Archive => plan.archive.push(cube.id),
                RetentionAction::Forget => plan.forget.push(cube.id),
            }
        }
        plan
    }
}

/// The result of a forgetting pass: which memories to keep, cool down, or drop.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionPlan {
    pub retain: Vec<uuid::Uuid>,
    pub archive: Vec<uuid::Uuid>,
    pub forget: Vec<uuid::Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MemoryPayload;

    fn cube_with(age_days: i64, accesses: u64, quality: f64, priority: Priority) -> MemCube {
        let mut c = MemCube::new(MemoryPayload::Text("hello".into()));
        c.metadata.last_accessed = Utc::now() - chrono::Duration::days(age_days);
        c.metadata.access_count = accesses;
        c.metadata.quality_score = quality;
        c.metadata.priority = priority;
        c
    }

    #[test]
    fn fresh_frequently_accessed_memory_is_retained() {
        let fc = ForgettingCurve::default();
        let cube = cube_with(0, 50, 0.9, Priority::High);
        let r = fc.retention(&cube, Utc::now());
        assert_eq!(r.action, RetentionAction::Retain);
        assert!(r.score > 0.5);
    }

    #[test]
    fn ancient_unused_memory_decays() {
        let fc = ForgettingCurve::default();
        let cube = cube_with(3650, 0, 0.0, Priority::Low);
        let r = fc.retention(&cube, Utc::now());
        assert!(r.recency < 0.01);
        assert!(matches!(
            r.action,
            RetentionAction::Forget | RetentionAction::Archive
        ));
    }

    #[test]
    fn reinforcement_extends_half_life() {
        let fc = ForgettingCurve::default();
        let rarely = cube_with(60, 1, 0.5, Priority::Normal);
        let often = cube_with(60, 500, 0.5, Priority::Normal);
        let r_rare = fc.retention(&rarely, Utc::now());
        let r_often = fc.retention(&often, Utc::now());
        assert!(r_often.recency > r_rare.recency);
    }

    #[test]
    fn critical_memories_never_forgotten() {
        let fc = ForgettingCurve::default();
        let cube = cube_with(100000, 0, 0.0, Priority::Critical);
        let plan = fc.classify(std::iter::once(&cube));
        assert_eq!(plan.retain.len(), 1);
        assert!(plan.forget.is_empty());
    }

    #[test]
    fn classify_partitions_memories() {
        let fc = ForgettingCurve::default();
        let hot = cube_with(0, 100, 1.0, Priority::High);
        let cold = cube_with(3650, 0, 0.0, Priority::Low);
        let cubes = vec![hot, cold];
        let plan = fc.classify(cubes.iter());
        assert_eq!(plan.retain.len(), 1);
        assert_eq!(plan.retain.len() + plan.archive.len() + plan.forget.len(), 2);
    }
}
