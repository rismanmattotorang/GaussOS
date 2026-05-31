// src/memory/episodic.rs
//! Columnar episodic store for fast time-range scans (Phase 2, #14).
//!
//! The L0 "raw" layer of agent memory is a high-volume, append-mostly stream of
//! events that is overwhelmingly queried by **time range** ("what happened last
//! Tuesday?", "the last 50 turns"). A row-of-structs store scans the whole log
//! for such queries. This store keeps the data **columnar** and **sorted by
//! time**, so range queries are a binary search + contiguous slice, and each
//! column stays cache-friendly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One episodic event (a returned, reconstructed row).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpisodicEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub namespace: String,
    pub content: String,
}

/// A time-sorted, columnar episodic event store.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EpisodicStore {
    // Parallel columns, kept sorted ascending by `timestamps`.
    timestamps: Vec<DateTime<Utc>>,
    ids: Vec<Uuid>,
    namespaces: Vec<String>,
    contents: Vec<String>,
}

impl EpisodicStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Append an event, preserving the time ordering. Appends at the end in O(1)
    /// when monotonic (the common case); otherwise inserts at the correct
    /// position to keep the columns sorted.
    pub fn append(&mut self, event: EpisodicEvent) {
        let pos = match self.timestamps.last() {
            Some(last) if event.timestamp >= *last => self.timestamps.len(),
            _ => self.timestamps.partition_point(|t| *t <= event.timestamp),
        };
        self.timestamps.insert(pos, event.timestamp);
        self.ids.insert(pos, event.id);
        self.namespaces.insert(pos, event.namespace);
        self.contents.insert(pos, event.content);
    }

    fn row(&self, i: usize) -> EpisodicEvent {
        EpisodicEvent {
            id: self.ids[i],
            timestamp: self.timestamps[i],
            namespace: self.namespaces[i].clone(),
            content: self.contents[i].clone(),
        }
    }

    /// All events in `[start, end)`, found via binary search over the time
    /// column — O(log n + matches), not O(n).
    pub fn range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<EpisodicEvent> {
        let lo = self.timestamps.partition_point(|t| *t < start);
        let hi = self.timestamps.partition_point(|t| *t < end);
        (lo..hi).map(|i| self.row(i)).collect()
    }

    /// The `n` most recent events (newest last).
    pub fn recent(&self, n: usize) -> Vec<EpisodicEvent> {
        let start = self.len().saturating_sub(n);
        (start..self.len()).map(|i| self.row(i)).collect()
    }

    /// Recent events restricted to a namespace.
    pub fn recent_in_namespace(&self, namespace: &str, n: usize) -> Vec<EpisodicEvent> {
        let mut out: Vec<EpisodicEvent> = (0..self.len())
            .rev()
            .filter(|&i| self.namespaces[i] == namespace)
            .take(n)
            .map(|i| self.row(i))
            .collect();
        out.reverse(); // chronological order
        out
    }

    /// Drop events older than `cutoff` (retention). Returns how many were removed.
    pub fn prune_before(&mut self, cutoff: DateTime<Utc>) -> usize {
        let cut = self.timestamps.partition_point(|t| *t < cutoff);
        if cut == 0 {
            return 0;
        }
        self.timestamps.drain(0..cut);
        self.ids.drain(0..cut);
        self.namespaces.drain(0..cut);
        self.contents.drain(0..cut);
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn ev(n: u128, secs: i64) -> EpisodicEvent {
        EpisodicEvent {
            id: Uuid::from_u128(n),
            timestamp: Utc::now() + Duration::seconds(secs),
            namespace: "conv".into(),
            content: format!("event {n}"),
        }
    }

    #[test]
    fn append_keeps_sorted_even_out_of_order() {
        let mut s = EpisodicStore::new();
        s.append(ev(1, 10));
        s.append(ev(2, 30));
        s.append(ev(3, 20)); // out of order
        let all = s.range(Utc::now() - Duration::seconds(1), Utc::now() + Duration::seconds(100));
        let order: Vec<u128> = all.iter().map(|e| e.id.as_u128()).collect();
        assert_eq!(order, vec![1, 3, 2]); // sorted by time: 10, 20, 30
    }

    #[test]
    fn range_query_is_bounded() {
        let mut s = EpisodicStore::new();
        for i in 0..10 {
            s.append(ev(i, i as i64 * 10));
        }
        let base = Utc::now();
        let res = s.range(base + Duration::seconds(25), base + Duration::seconds(55));
        // expect events at 30, 40, 50 → ids 3,4,5
        let ids: Vec<u128> = res.iter().map(|e| e.id.as_u128()).collect();
        assert_eq!(ids, vec![3, 4, 5]);
    }

    #[test]
    fn recent_and_namespace() {
        let mut s = EpisodicStore::new();
        for i in 0..5 {
            let mut e = ev(i, i as i64);
            if i % 2 == 0 {
                e.namespace = "other".into();
            }
            s.append(e);
        }
        assert_eq!(s.recent(2).len(), 2);
        assert_eq!(s.recent(2)[1].id, Uuid::from_u128(4));
        // namespaces "conv" are ids 1,3
        let conv = s.recent_in_namespace("conv", 10);
        assert_eq!(conv.iter().map(|e| e.id.as_u128()).collect::<Vec<_>>(), vec![1, 3]);
    }

    #[test]
    fn prune_before_drops_old() {
        let mut s = EpisodicStore::new();
        for i in 0..10 {
            s.append(ev(i, i as i64 * 10));
        }
        let removed = s.prune_before(Utc::now() + Duration::seconds(45));
        assert_eq!(removed, 5); // 0,10,20,30,40
        assert_eq!(s.len(), 5);
    }
}
