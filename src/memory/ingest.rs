// src/memory/ingest.rs
//! LLM-driven extract → update ingestion (Phase 1, roadmap #3).
//!
//! This is the Mem0/Zep-class memory-formation loop: turn raw conversation
//! turns into durable, de-duplicated, temporally-correct knowledge.
//!
//! Pipeline:
//! 1. **Extract** candidate facts (`subject, predicate, object`) from messages.
//!    Uses the configured multi-provider LLM ([`crate::agents::llm`]) when a key
//!    is present, and a deterministic heuristic extractor otherwise — so the
//!    pipeline always works offline and is fully testable.
//! 2. **Resolve time** — relative/absolute expressions in the turn become the
//!    fact's `valid_at` ([`super::temporal_parse`]).
//! 3. **Decide & apply** ADD / UPDATE / NOOP against the bi-temporal store: an
//!    identical live fact is a NOOP; a conflicting one is superseded (UPDATE,
//!    preserving history); otherwise it is ADDed. Negations ("no longer …")
//!    invalidate the matching fact (DELETE).
//!
//! The decision/storage layer reuses [`super::temporal::TemporalFactStore`],
//! which already implements correct, auditable supersession.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::agents::llm::{ChatTurn, LlmClient};
use crate::core::{Message, MessageRole};
use crate::error::Result;
use crate::memory::temporal::{TemporalFact, TemporalFactStore};
use crate::memory::temporal_parse;

/// A candidate fact extracted from conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<DateTime<Utc>>,
    /// True when the statement negates/retracts the fact ("no longer …").
    #[serde(default)]
    pub negated: bool,
}

fn default_confidence() -> f32 {
    0.8
}

/// What happened to a single extracted fact during ingestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateAction {
    Add,
    Update { superseded: usize },
    Delete,
    Noop,
}

/// Summary of an ingestion run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestReport {
    pub extracted: usize,
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
    pub noop: usize,
    pub actions: Vec<(ExtractedFact, UpdateAction)>,
}

/// Configuration for the ingestor.
#[derive(Debug, Clone)]
pub struct IngestConfig {
    /// Drop extracted facts below this confidence.
    pub min_confidence: f32,
    /// Prefer the LLM extractor when an API key is configured.
    pub use_llm: bool,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self { min_confidence: 0.3, use_llm: true }
    }
}

/// The ingestion engine.
#[derive(Debug, Clone, Default)]
pub struct MemoryIngestor {
    config: IngestConfig,
}

impl MemoryIngestor {
    pub fn new(config: IngestConfig) -> Self {
        Self { config }
    }

    /// Extract candidate facts from messages, via the LLM when configured and
    /// requested, otherwise via the deterministic heuristic extractor.
    pub async fn extract(&self, messages: &[Message], llm: &LlmClient) -> Vec<ExtractedFact> {
        if self.config.use_llm && llm.is_configured() {
            if let Ok(facts) = self.extract_llm(messages, llm).await {
                if !facts.is_empty() {
                    return facts;
                }
            }
            // Fall through to heuristic if the LLM returned nothing/unparseable.
        }
        self.extract_heuristic(messages)
    }

    /// Full pipeline: extract → resolve time → apply to the bi-temporal store.
    pub async fn ingest(
        &self,
        store: &mut TemporalFactStore,
        messages: &[Message],
        llm: &LlmClient,
    ) -> IngestReport {
        let facts = self.extract(messages, llm).await;
        self.apply_all(store, facts)
    }

    /// Apply already-extracted facts to the store (synchronous). Separated from
    /// [`Self::extract`] so callers can run extraction (which may await an LLM)
    /// without holding a store lock across the await point.
    pub fn apply_all(&self, store: &mut TemporalFactStore, facts: Vec<ExtractedFact>) -> IngestReport {
        let mut report = IngestReport { extracted: facts.len(), ..Default::default() };
        for fact in facts {
            if fact.confidence < self.config.min_confidence {
                continue;
            }
            let action = self.apply(store, &fact);
            match &action {
                UpdateAction::Add => report.added += 1,
                UpdateAction::Update { .. } => report.updated += 1,
                UpdateAction::Delete => report.deleted += 1,
                UpdateAction::Noop => report.noop += 1,
            }
            report.actions.push((fact, action));
        }
        report
    }

    /// Decide and apply ADD/UPDATE/DELETE/NOOP for one fact.
    fn apply(&self, store: &mut TemporalFactStore, fact: &ExtractedFact) -> UpdateAction {
        // Negation → invalidate the current matching value (DELETE).
        if fact.negated {
            let now = fact.valid_at.unwrap_or_else(Utc::now);
            let ids: Vec<_> = store
                .current_value(&fact.subject, &fact.predicate)
                .into_iter()
                .filter(|f| f.object == fact.object || fact.object.is_empty())
                .map(|f| f.id)
                .collect();
            if ids.is_empty() {
                return UpdateAction::Noop;
            }
            for id in &ids {
                store.invalidate(id, now);
            }
            return UpdateAction::Delete;
        }

        // Identical live fact already present → NOOP.
        let already = store
            .current_value(&fact.subject, &fact.predicate)
            .into_iter()
            .any(|f| f.object.eq_ignore_ascii_case(&fact.object));
        if already {
            return UpdateAction::Noop;
        }

        let mut tf = TemporalFact::new(&fact.subject, &fact.predicate, &fact.object)
            .with_confidence(fact.confidence);
        if let Some(v) = fact.valid_at {
            tf = tf.with_valid_from(v);
        }
        let report = store.ingest(tf);
        if report.superseded.is_empty() {
            UpdateAction::Add
        } else {
            UpdateAction::Update { superseded: report.superseded.len() }
        }
    }

    // ---- Heuristic extraction (deterministic, offline) ----

    fn extract_heuristic(&self, messages: &[Message]) -> Vec<ExtractedFact> {
        let mut out = Vec::new();
        for msg in messages {
            // Only mine user statements for durable facts.
            if !matches!(msg.role, MessageRole::User) {
                continue;
            }
            let valid_at = temporal_parse::resolve(&msg.content, msg.timestamp);
            for sentence in split_sentences(&msg.content) {
                let negated = NEGATION.is_match(&sentence);
                // Strip negation words so the SVO pattern still matches
                // ("I no longer work at X" → "I work at X"), keeping the flag.
                let cleaned = if negated {
                    collapse_ws(&NEGATION.replace_all(&sentence, " "))
                } else {
                    sentence.clone()
                };
                for mut f in match_all(&cleaned) {
                    f.negated = negated;
                    f.valid_at = valid_at;
                    out.push(f);
                }
            }
        }
        out
    }

    // ---- LLM extraction ----

    async fn extract_llm(&self, messages: &[Message], llm: &LlmClient) -> Result<Vec<ExtractedFact>> {
        let system = "You extract durable facts from a conversation as a JSON array. \
            Each item: {\"subject\":\"\",\"predicate\":\"\",\"object\":\"\",\"confidence\":0..1,\"negated\":bool}. \
            Use 'user' as the subject for first-person statements. Output ONLY the JSON array.";
        let convo: String = messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        let turns = vec![ChatTurn::new("user", convo)];
        let text = llm.complete(system, &turns).await?;
        Ok(parse_facts_json(&text))
    }
}

/// Parse the model's response into facts, tolerating code fences / prose.
fn parse_facts_json(text: &str) -> Vec<ExtractedFact> {
    let start = text.find('[');
    let end = text.rfind(']');
    if let (Some(s), Some(e)) = (start, end) {
        if e > s {
            if let Ok(facts) = serde_json::from_str::<Vec<ExtractedFact>>(&text[s..=e]) {
                return facts;
            }
        }
    }
    Vec::new()
}

fn split_sentences(text: &str) -> Vec<String> {
    text.split(|c| c == '.' || c == '!' || c == '?' || c == '\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// Compiled once. Each pattern maps to (subject_is_user, predicate).
static PATTERNS: Lazy<Vec<(Regex, &'static str, bool)>> = Lazy::new(|| {
    vec![
        (Regex::new(r"(?i)\bmy name is (?P<o>[A-Za-z][\w' ]+)").unwrap(), "name", true),
        (Regex::new(r"(?i)\bI (?:work|worked) (?:at|for) (?P<o>[\w' ]+)").unwrap(), "works_at", true),
        (Regex::new(r"(?i)\bI (?:live|lived) in (?P<o>[\w' ]+)").unwrap(), "lives_in", true),
        (Regex::new(r"(?i)\bI (?:like|love|prefer|enjoy) (?P<o>[\w' ]+)").unwrap(), "likes", true),
        (Regex::new(r"(?i)\bI am (?:a |an )?(?P<o>[\w' ]+)").unwrap(), "is", true),
        (Regex::new(r"(?i)(?P<s>[A-Z][\w']+) works (?:at|for) (?P<o>[\w' ]+)").unwrap(), "works_at", false),
    ]
});

static NEGATION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(no longer|not anymore|don't|do not|stopped)\b").unwrap());

/// Collapse runs of whitespace to single spaces and trim.
fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Return every fact matched by any pattern in `sentence` (negation handled by
/// the caller). A sentence like "I work at Kalbe and I love coffee" yields two.
fn match_all(sentence: &str) -> Vec<ExtractedFact> {
    let mut facts = Vec::new();
    for (re, predicate, subject_is_user) in PATTERNS.iter() {
        if let Some(caps) = re.captures(sentence) {
            let Some(obj_match) = caps.name("o") else { continue };
            let object = clean_object(obj_match.as_str());
            if object.is_empty() {
                continue;
            }
            let subject = if *subject_is_user {
                "user".to_string()
            } else {
                caps.name("s").map(|m| m.as_str().to_string()).unwrap_or_else(|| "user".into())
            };
            facts.push(ExtractedFact {
                subject,
                predicate: predicate.to_string(),
                object,
                confidence: 0.75,
                valid_at: None,
                negated: false,
            });
        }
    }
    facts
}

/// Trim an extracted object phrase to a clean noun phrase.
fn clean_object(raw: &str) -> String {
    let lowered_cutters = [" and ", " but ", " because ", " since ", " when ", " who ", " which "];
    let mut s = raw.trim().to_string();
    let lower = s.to_lowercase();
    let mut cut = s.len();
    for c in lowered_cutters {
        if let Some(idx) = lower.find(c) {
            cut = cut.min(idx);
        }
    }
    s.truncate(cut);
    s.trim().trim_end_matches([',', '.', ';', ':']).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::llm::LlmClient;

    fn user_msg(content: &str) -> Message {
        Message { role: MessageRole::User, content: content.to_string(), timestamp: Utc::now(), metadata: None }
    }

    fn unconfigured_llm() -> LlmClient {
        for k in ["LLM_API_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY", "LLM_PROVIDER"] {
            std::env::remove_var(k);
        }
        LlmClient::from_env()
    }

    #[tokio::test]
    async fn heuristic_extracts_first_person_facts() {
        let ing = MemoryIngestor::default();
        let msgs = vec![user_msg("Hi, my name is Edwin. I work at Kalbe and I love coffee.")];
        let facts = ing.extract(&msgs, &unconfigured_llm()).await;
        assert!(facts.iter().any(|f| f.predicate == "name" && f.object.starts_with("Edwin")));
        assert!(facts.iter().any(|f| f.predicate == "works_at" && f.object.contains("Kalbe")));
        assert!(facts.iter().any(|f| f.predicate == "likes" && f.object.contains("coffee")));
    }

    #[tokio::test]
    async fn ingest_add_update_noop() {
        let ing = MemoryIngestor::default();
        let mut store = TemporalFactStore::new();
        let llm = unconfigured_llm();

        // First mention: ADD.
        let r1 = ing.ingest(&mut store, &[user_msg("I work at OldCorp")], &llm).await;
        assert_eq!(r1.added, 1);

        // Same fact again: NOOP.
        let r2 = ing.ingest(&mut store, &[user_msg("I work at OldCorp")], &llm).await;
        assert_eq!(r2.noop, 1);

        // Conflicting value: UPDATE (supersede), history preserved.
        let r3 = ing.ingest(&mut store, &[user_msg("I work at Kalbe")], &llm).await;
        assert_eq!(r3.updated, 1);
        assert_eq!(store.current_value("user", "works_at").len(), 1);
        assert_eq!(store.current_value("user", "works_at")[0].object, "Kalbe");
        assert_eq!(store.history("user", "works_at").len(), 2);
    }

    #[tokio::test]
    async fn negation_deletes() {
        let ing = MemoryIngestor::default();
        let mut store = TemporalFactStore::new();
        let llm = unconfigured_llm();
        ing.ingest(&mut store, &[user_msg("I work at Kalbe")], &llm).await;
        let r = ing.ingest(&mut store, &[user_msg("I no longer work at Kalbe")], &llm).await;
        assert!(r.deleted >= 1 || r.noop >= 1);
        // After invalidation, no current value remains.
        assert!(store.current_value("user", "works_at").is_empty());
    }

    #[tokio::test]
    async fn temporal_expression_sets_valid_at() {
        let ing = MemoryIngestor::default();
        let msgs = vec![user_msg("I work at Kalbe. I started two weeks ago.")];
        let facts = ing.extract(&msgs, &unconfigured_llm()).await;
        assert!(facts.iter().any(|f| f.valid_at.is_some()));
    }

    #[test]
    fn parse_facts_json_tolerates_fences() {
        let text = "Here you go:\n```json\n[{\"subject\":\"user\",\"predicate\":\"likes\",\"object\":\"tea\"}]\n```";
        let facts = parse_facts_json(text);
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].object, "tea");
    }
}
