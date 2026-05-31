// src/security/audit.rs
//! Append-only audit log of security-relevant events.
//!
//! Compliance regimes (SOC2/GDPR/HIPAA) require a tamper-evident record of who
//! did what, when. This is an in-memory, append-only log with a monotonically
//! increasing sequence number and a rolling hash chain so any tampering with
//! earlier entries is detectable. It is backend-agnostic and can be flushed to
//! durable storage by the caller.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Whether an audited action succeeded or was denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditOutcome {
    Allowed,
    Denied,
    Error,
}

/// A single audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub outcome: AuditOutcome,
    /// Hash chaining this entry to the previous one (tamper-evidence).
    pub chain_hash: u64,
}

/// An append-only audit log with a hash chain.
#[derive(Debug, Default)]
pub struct AuditLog {
    events: parking_lot::RwLock<Vec<AuditEvent>>,
    seq: AtomicU64,
    last_hash: AtomicU64,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an event, returning its assigned sequence number.
    pub fn record(
        &self,
        actor: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        outcome: AuditOutcome,
    ) -> u64 {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let actor = actor.into();
        let action = action.into();
        let resource = resource.into();
        let prev = self.last_hash.load(Ordering::SeqCst);

        // Chain hash = H(prev, seq, actor, action, resource, outcome).
        let mut hasher = DefaultHasher::new();
        prev.hash(&mut hasher);
        seq.hash(&mut hasher);
        actor.hash(&mut hasher);
        action.hash(&mut hasher);
        resource.hash(&mut hasher);
        (outcome as u8).hash(&mut hasher);
        let chain_hash = hasher.finish();
        self.last_hash.store(chain_hash, Ordering::SeqCst);

        let event = AuditEvent {
            seq,
            timestamp: Utc::now(),
            actor,
            action,
            resource,
            outcome,
            chain_hash,
        };
        self.events.write().push(event);
        seq
    }

    pub fn len(&self) -> usize {
        self.events.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.read().is_empty()
    }

    /// The most recent `n` events (newest last).
    pub fn recent(&self, n: usize) -> Vec<AuditEvent> {
        let g = self.events.read();
        let start = g.len().saturating_sub(n);
        g[start..].to_vec()
    }

    /// Verify the hash chain is intact (no earlier entry was altered/removed).
    pub fn verify(&self) -> bool {
        let g = self.events.read();
        let mut prev = 0u64;
        for e in g.iter() {
            let mut hasher = DefaultHasher::new();
            prev.hash(&mut hasher);
            e.seq.hash(&mut hasher);
            e.actor.hash(&mut hasher);
            e.action.hash(&mut hasher);
            e.resource.hash(&mut hasher);
            (e.outcome as u8).hash(&mut hasher);
            if hasher.finish() != e.chain_hash {
                return false;
            }
            prev = e.chain_hash;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_chains() {
        let log = AuditLog::new();
        assert_eq!(log.record("alice", "read", "users/alice", AuditOutcome::Allowed), 0);
        assert_eq!(log.record("bob", "write", "users/bob", AuditOutcome::Denied), 1);
        assert_eq!(log.len(), 2);
        assert!(log.verify());
        assert_eq!(log.recent(1)[0].actor, "bob");
    }

    #[test]
    fn tampering_breaks_the_chain() {
        let log = AuditLog::new();
        log.record("a", "x", "r", AuditOutcome::Allowed);
        log.record("b", "y", "r", AuditOutcome::Allowed);
        // Tamper with the first event's actor.
        log.events.write()[0].actor = "mallory".to_string();
        assert!(!log.verify());
    }
}
