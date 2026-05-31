// src/security/mod.rs
//! Security & compliance for GaussOS (Phase 5, roadmap #29).
//!
//! * [`audit`] — append-only audit log of security-relevant events.
//! * [`rls`] — per-namespace row-level access policies.
//! * [`encryption`] — field-level AES-256-GCM encryption with key rotation
//!   (compiled only with the `encryption` feature).

pub mod audit;
pub mod rls;

#[cfg(feature = "encryption")]
pub mod encryption;

pub use audit::{AuditEvent, AuditLog, AuditOutcome};
pub use rls::{Action, Principal, RlsEngine, RlsPolicy};

#[cfg(feature = "encryption")]
pub use encryption::{EncryptedField, FieldEncryptor};
