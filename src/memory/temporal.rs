// src/memory/temporal.rs
//! Bi-temporal knowledge model for GaussOS.
//!
//! Agent memory is rarely static: facts are corrected, superseded, and become
//! stale. Most vector stores handle this by *deleting* old data, which destroys
//! history and makes "what did the agent believe last Tuesday?" unanswerable.
//!
//! Following the bi-temporal design pioneered by Zep/Graphiti, GaussOS tracks
//! **two independent time axes** for every fact:
//!
//! * **Valid time** (`valid_at` / `invalid_at`) — the interval during which the
//!   fact was true *in the world*.
//! * **Transaction time** (`recorded_at` / `expired_at`) — the interval during
//!   which the fact was *known to the system*.
//!
//! This lets the engine answer point-in-time queries, reason about retroactive
//! corrections, and **invalidate rather than delete** superseded knowledge —
//! preserving a fully auditable history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A single knowledge assertion, modelled as a subject–predicate–object triple
/// with bi-temporal validity. Triples form the edges of the agent's evolving
/// knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFact {
    pub id: Uuid,
    /// Entity the fact is about, e.g. `"user:alice"`.
    pub subject: String,
    /// Relation / attribute, e.g. `"works_at"`.
    pub predicate: String,
    /// Value, e.g. `"Kalbe"`.
    pub object: String,

    /// When the fact became true in the world.
    pub valid_at: DateTime<Utc>,
    /// When the fact stopped being true in the world (`None` = still valid).
    pub invalid_at: Option<DateTime<Utc>>,

    /// When the system first recorded the fact.
    pub recorded_at: DateTime<Utc>,
    /// When the system superseded/retracted the fact (`None` = live record).
    pub expired_at: Option<DateTime<Utc>>,

    /// Extraction confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Memory cube(s) this fact was derived from (provenance).
    pub source_memory: Option<Uuid>,
}

impl TemporalFact {
    /// Create a new, currently-valid fact recorded as of `now`.
    pub fn new(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            valid_at: now,
            invalid_at: None,
            recorded_at: now,
            expired_at: None,
            confidence: 1.0,
            source_memory: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_valid_from(mut self, valid_at: DateTime<Utc>) -> Self {
        self.valid_at = valid_at;
        self
    }

    pub fn with_source(mut self, memory_id: Uuid) -> Self {
        self.source_memory = Some(memory_id);
        self
    }

    /// True if the system still considers this record live (not expired).
    pub fn is_live(&self) -> bool {
        self.expired_at.is_none()
    }

    /// True if the fact held true in the world at `at` (valid-time query),
    /// regardless of whether the system has since expired the record.
    pub fn was_valid_at(&self, at: DateTime<Utc>) -> bool {
        at >= self.valid_at && self.invalid_at.map(|iv| at < iv).unwrap_or(true)
    }

    /// True if the system knew this fact at transaction time `at`.
    pub fn was_known_at(&self, at: DateTime<Utc>) -> bool {
        at >= self.recorded_at && self.expired_at.map(|ex| at < ex).unwrap_or(true)
    }

    /// `(subject, predicate)` key used to detect conflicting assertions.
    pub fn attribute_key(&self) -> (String, String) {
        (self.subject.clone(), self.predicate.clone())
    }
}

/// Outcome of ingesting a new fact, exposing exactly what changed so callers
/// can audit the knowledge-graph mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestReport {
    pub added: Uuid,
    /// Facts that were invalidated/expired because the new fact superseded them.
    pub superseded: Vec<Uuid>,
}

/// An append-only, bi-temporal store of [`TemporalFact`]s.
///
/// Nothing is ever physically removed during normal operation: superseding a
/// fact marks the old record `expired_at` (transaction time) and `invalid_at`
/// (valid time), keeping a complete audit trail.
#[derive(Debug, Default)]
pub struct TemporalFactStore {
    facts: HashMap<Uuid, TemporalFact>,
}

impl TemporalFactStore {
    pub fn new() -> Self {
        Self {
            facts: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.facts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Insert a fact without any conflict resolution.
    pub fn insert(&mut self, fact: TemporalFact) -> Uuid {
        let id = fact.id;
        self.facts.insert(id, fact);
        id
    }

    /// Ingest a fact, automatically superseding any *live* fact sharing the same
    /// `(subject, predicate)` with a different `object`. The superseded record's
    /// `invalid_at` is set to the new fact's `valid_at`, and its `expired_at` to
    /// now — preserving history while keeping a single current truth.
    pub fn ingest(&mut self, fact: TemporalFact) -> IngestReport {
        let now = Utc::now();
        let key = fact.attribute_key();
        let new_object = fact.object.clone();
        let new_valid_at = fact.valid_at;

        let mut superseded = Vec::new();
        for existing in self.facts.values_mut() {
            if existing.is_live()
                && existing.attribute_key() == key
                && existing.object != new_object
            {
                existing.expired_at = Some(now);
                if existing.invalid_at.is_none() {
                    // The prior fact stops being valid when the new one takes
                    // effect — but never before it itself became valid, so a
                    // backdated correction can't produce invalid_at < valid_at
                    // (which would make the record unsatisfiable in every query).
                    existing.invalid_at = Some(new_valid_at.max(existing.valid_at));
                }
                superseded.push(existing.id);
            }
        }

        let added = self.insert(fact);
        IngestReport { added, superseded }
    }

    /// Explicitly invalidate a fact in the world as of `at` (it remains a live
    /// system record describing a now-false past state).
    pub fn invalidate(&mut self, id: &Uuid, at: DateTime<Utc>) -> bool {
        if let Some(fact) = self.facts.get_mut(id) {
            fact.invalid_at = Some(at);
            true
        } else {
            false
        }
    }

    /// Retract a fact from the system entirely (transaction-time expiry).
    pub fn expire(&mut self, id: &Uuid) -> bool {
        if let Some(fact) = self.facts.get_mut(id) {
            fact.expired_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    pub fn get(&self, id: &Uuid) -> Option<&TemporalFact> {
        self.facts.get(id)
    }

    /// All facts the system currently believes are true (live + currently valid).
    pub fn current_facts(&self) -> Vec<&TemporalFact> {
        let now = Utc::now();
        self.facts
            .values()
            .filter(|f| f.is_live() && f.was_valid_at(now))
            .collect()
    }

    /// Point-in-time valid-time query: facts that held true at `at` according to
    /// records the system still keeps live.
    pub fn facts_valid_at(&self, at: DateTime<Utc>) -> Vec<&TemporalFact> {
        self.facts
            .values()
            .filter(|f| f.is_live() && f.was_valid_at(at))
            .collect()
    }

    /// "As the system knew it then" query across both time axes — reconstruct
    /// the agent's beliefs at transaction time `as_of` about world time `at`.
    pub fn facts_as_known_at(
        &self,
        as_of: DateTime<Utc>,
        at: DateTime<Utc>,
    ) -> Vec<&TemporalFact> {
        self.facts
            .values()
            .filter(|f| f.was_known_at(as_of) && f.was_valid_at(at))
            .collect()
    }

    /// Current value(s) of a `(subject, predicate)` attribute.
    pub fn current_value(&self, subject: &str, predicate: &str) -> Vec<&TemporalFact> {
        let now = Utc::now();
        self.facts
            .values()
            .filter(|f| {
                f.is_live()
                    && f.was_valid_at(now)
                    && f.subject == subject
                    && f.predicate == predicate
            })
            .collect()
    }

    /// Full, ordered history of an attribute (newest valid_at first), including
    /// superseded records — the audit trail for "how did this change over time?".
    pub fn history(&self, subject: &str, predicate: &str) -> Vec<&TemporalFact> {
        let mut hist: Vec<&TemporalFact> = self
            .facts
            .values()
            .filter(|f| f.subject == subject && f.predicate == predicate)
            .collect();
        hist.sort_by(|a, b| b.valid_at.cmp(&a.valid_at));
        hist
    }

    /// All live facts about a subject (its current neighbourhood in the graph).
    pub fn facts_about(&self, subject: &str) -> Vec<&TemporalFact> {
        let now = Utc::now();
        self.facts
            .values()
            .filter(|f| f.is_live() && f.was_valid_at(now) && f.subject == subject)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_supersedes_conflicting_fact() {
        let mut store = TemporalFactStore::new();
        let r1 = store.ingest(TemporalFact::new("user:alice", "works_at", "OldCorp"));
        let r2 = store.ingest(TemporalFact::new("user:alice", "works_at", "Kalbe"));

        assert!(r2.superseded.contains(&r1.added));
        let current = store.current_value("user:alice", "works_at");
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].object, "Kalbe");
    }

    #[test]
    fn same_object_is_not_superseded() {
        let mut store = TemporalFactStore::new();
        store.ingest(TemporalFact::new("user:alice", "likes", "coffee"));
        let r2 = store.ingest(TemporalFact::new("user:alice", "likes", "coffee"));
        assert!(r2.superseded.is_empty());
    }

    #[test]
    fn different_predicates_coexist() {
        let mut store = TemporalFactStore::new();
        store.ingest(TemporalFact::new("user:alice", "works_at", "Kalbe"));
        store.ingest(TemporalFact::new("user:alice", "lives_in", "Jakarta"));
        assert_eq!(store.facts_about("user:alice").len(), 2);
    }

    #[test]
    fn history_preserves_superseded_records() {
        let mut store = TemporalFactStore::new();
        store.ingest(TemporalFact::new("user:alice", "works_at", "OldCorp"));
        store.ingest(TemporalFact::new("user:alice", "works_at", "Kalbe"));
        let hist = store.history("user:alice", "works_at");
        assert_eq!(hist.len(), 2);
        // Newest first.
        assert_eq!(hist[0].object, "Kalbe");
    }

    #[test]
    fn point_in_time_valid_query() {
        let mut store = TemporalFactStore::new();
        let past = Utc::now() - chrono::Duration::days(10);
        let old = TemporalFact::new("user:alice", "title", "Engineer").with_valid_from(past);
        store.insert(old);
        // A fact that becomes valid tomorrow should not appear "now".
        let future = TemporalFact::new("user:alice", "title", "Manager")
            .with_valid_from(Utc::now() + chrono::Duration::days(1));
        store.insert(future);

        let now_valid = store.facts_valid_at(Utc::now());
        assert_eq!(now_valid.len(), 1);
        assert_eq!(now_valid[0].object, "Engineer");
    }
}
