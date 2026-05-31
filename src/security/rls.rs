// src/security/rls.rs
//! Per-namespace row-level security (RLS) policies.
//!
//! Memories are organised by hierarchical namespace (`users/alice/work`). RLS
//! lets you grant a principal access to a namespace **and its descendants**,
//! with per-action (read/write/delete/admin) granularity, plus an explicit
//! deny that overrides grants. Default is deny.

use serde::{Deserialize, Serialize};

/// Actions that can be authorised against a namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Read,
    Write,
    Delete,
    Admin,
}

/// A principal (user / service / agent) identified by id, optionally with roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub id: String,
    pub roles: Vec<String>,
}

impl Principal {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), roles: Vec::new() }
    }
    pub fn with_roles(id: impl Into<String>, roles: Vec<String>) -> Self {
        Self { id: id.into(), roles }
    }
}

/// A single policy rule. `subject` matches a principal id or `role:<name>` or
/// `*` (everyone). `namespace` is a prefix that also covers descendants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlsPolicy {
    pub subject: String,
    pub namespace: String,
    pub actions: Vec<Action>,
    /// Deny rules override allow rules.
    pub deny: bool,
}

impl RlsPolicy {
    pub fn allow(subject: impl Into<String>, namespace: impl Into<String>, actions: Vec<Action>) -> Self {
        Self { subject: subject.into(), namespace: namespace.into(), actions, deny: false }
    }
    pub fn deny(subject: impl Into<String>, namespace: impl Into<String>, actions: Vec<Action>) -> Self {
        Self { subject: subject.into(), namespace: namespace.into(), actions, deny: true }
    }

    fn matches_subject(&self, p: &Principal) -> bool {
        self.subject == "*"
            || self.subject == p.id
            || self
                .subject
                .strip_prefix("role:")
                .map(|r| p.roles.iter().any(|pr| pr == r))
                .unwrap_or(false)
    }

    fn matches_namespace(&self, ns: &str) -> bool {
        ns == self.namespace || ns.starts_with(&format!("{}/", self.namespace)) || self.namespace == "*"
    }
}

/// Evaluates RLS policies. Default-deny; an explicit deny beats any allow.
#[derive(Debug, Default)]
pub struct RlsEngine {
    policies: Vec<RlsPolicy>,
}

impl RlsEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, policy: RlsPolicy) -> &mut Self {
        self.policies.push(policy);
        self
    }

    /// Is `principal` allowed to perform `action` on `namespace`?
    pub fn can(&self, principal: &Principal, action: Action, namespace: &str) -> bool {
        let mut allowed = false;
        for p in &self.policies {
            if p.matches_subject(principal)
                && p.matches_namespace(namespace)
                && p.actions.contains(&action)
            {
                if p.deny {
                    return false; // explicit deny wins immediately
                }
                allowed = true;
            }
        }
        allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_deny() {
        let engine = RlsEngine::new();
        assert!(!engine.can(&Principal::new("alice"), Action::Read, "users/alice"));
    }

    #[test]
    fn grant_covers_descendants() {
        let mut e = RlsEngine::new();
        e.add(RlsPolicy::allow("alice", "users/alice", vec![Action::Read, Action::Write]));
        assert!(e.can(&Principal::new("alice"), Action::Read, "users/alice"));
        assert!(e.can(&Principal::new("alice"), Action::Read, "users/alice/work"));
        assert!(!e.can(&Principal::new("alice"), Action::Delete, "users/alice"));
        assert!(!e.can(&Principal::new("alice"), Action::Read, "users/bob"));
    }

    #[test]
    fn role_based_and_wildcard() {
        let mut e = RlsEngine::new();
        e.add(RlsPolicy::allow("role:admin", "*", vec![Action::Read, Action::Admin]));
        let admin = Principal::with_roles("svc", vec!["admin".into()]);
        assert!(e.can(&admin, Action::Admin, "anything/here"));
        assert!(!e.can(&Principal::new("nobody"), Action::Admin, "anything/here"));
    }

    #[test]
    fn explicit_deny_overrides_allow() {
        let mut e = RlsEngine::new();
        e.add(RlsPolicy::allow("*", "public", vec![Action::Read]));
        e.add(RlsPolicy::deny("blocked", "public", vec![Action::Read]));
        assert!(e.can(&Principal::new("anyone"), Action::Read, "public"));
        assert!(!e.can(&Principal::new("blocked"), Action::Read, "public"));
    }
}
