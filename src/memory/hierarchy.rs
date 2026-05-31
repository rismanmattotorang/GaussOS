// src/memory/hierarchy.rs
//! Hierarchical memory layering for GaussOS (the L0→L3 pyramid).
//!
//! Flat memory stores force agents to choose between *cost* (stuffing raw logs
//! into context) and *fidelity* (lossy summarisation). TencentDB-Agent-Memory's
//! key insight is **progressive disclosure**: organise memory into a pyramid
//! where agents normally operate on condensed top layers but can always drill
//! down to ground-truth evidence.
//!
//! GaussOS implements four layers, each node linking to the nodes it was
//! derived from so provenance is never lost:
//!
//! * **L0 — Raw**: verbatim messages / tool outputs.
//! * **L1 — Atomic facts**: discrete, self-contained statements distilled from L0.
//! * **L2 — Scenario blocks**: thematic aggregations of related L1 facts.
//! * **L3 — Persona**: a synthesised profile of stable preferences & patterns.
//!
//! Because every higher node carries `derived_from` references, the engine
//! offers *lossless* drill-down — the abstraction is reversible all the way
//! back to the originating message.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::core::{Message, MessageRole};

/// The layer a memory node lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLayer {
    /// L0 — raw, verbatim content.
    Raw,
    /// L1 — atomic facts.
    Atomic,
    /// L2 — scenario / episode blocks.
    Scenario,
    /// L3 — synthesised persona.
    Persona,
}

impl MemoryLayer {
    pub fn level(&self) -> u8 {
        match self {
            MemoryLayer::Raw => 0,
            MemoryLayer::Atomic => 1,
            MemoryLayer::Scenario => 2,
            MemoryLayer::Persona => 3,
        }
    }
}

/// A node in the hierarchical memory pyramid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerNode {
    pub id: Uuid,
    pub layer: MemoryLayer,
    /// Condensed content appropriate to the layer.
    pub content: String,
    /// References to the lower-layer nodes this node was distilled from.
    pub derived_from: Vec<Uuid>,
    /// Free-form tags / topic labels for grouping & retrieval.
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    /// Confidence/quality of the distillation in `[0.0, 1.0]`.
    pub confidence: f32,
}

impl LayerNode {
    fn new(layer: MemoryLayer, content: String, derived_from: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            layer,
            content,
            derived_from,
            tags: Vec::new(),
            created_at: Utc::now(),
            confidence: 1.0,
        }
    }
}

/// The full layered memory pyramid with provenance links between layers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryHierarchy {
    nodes: HashMap<Uuid, LayerNode>,
    raw: Vec<Uuid>,
    atomic: Vec<Uuid>,
    scenario: Vec<Uuid>,
    persona: Vec<Uuid>,
}

impl MemoryHierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, node: LayerNode) -> Uuid {
        let id = node.id;
        match node.layer {
            MemoryLayer::Raw => self.raw.push(id),
            MemoryLayer::Atomic => self.atomic.push(id),
            MemoryLayer::Scenario => self.scenario.push(id),
            MemoryLayer::Persona => self.persona.push(id),
        }
        self.nodes.insert(id, node);
        id
    }

    pub fn get(&self, id: &Uuid) -> Option<&LayerNode> {
        self.nodes.get(id)
    }

    pub fn layer(&self, layer: MemoryLayer) -> Vec<&LayerNode> {
        let ids = match layer {
            MemoryLayer::Raw => &self.raw,
            MemoryLayer::Atomic => &self.atomic,
            MemoryLayer::Scenario => &self.scenario,
            MemoryLayer::Persona => &self.persona,
        };
        ids.iter().filter_map(|id| self.nodes.get(id)).collect()
    }

    pub fn total_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Add a raw (L0) node from a verbatim message.
    pub fn add_raw(&mut self, content: impl Into<String>) -> Uuid {
        self.add(LayerNode::new(MemoryLayer::Raw, content.into(), vec![]))
    }

    /// Add an atomic (L1) fact derived from one or more raw nodes.
    pub fn add_atomic(&mut self, content: impl Into<String>, from: Vec<Uuid>) -> Uuid {
        self.add(LayerNode::new(MemoryLayer::Atomic, content.into(), from))
    }

    /// Add a scenario (L2) block aggregating atomic facts.
    pub fn add_scenario(&mut self, content: impl Into<String>, from: Vec<Uuid>) -> Uuid {
        self.add(LayerNode::new(MemoryLayer::Scenario, content.into(), from))
    }

    /// Add / replace the persona (L3) synthesis.
    pub fn add_persona(&mut self, content: impl Into<String>, from: Vec<Uuid>) -> Uuid {
        self.add(LayerNode::new(MemoryLayer::Persona, content.into(), from))
    }

    /// Progressive disclosure: return the top-most populated layer as the
    /// compact context an agent should read first.
    pub fn top_context(&self) -> Vec<&LayerNode> {
        for layer in [
            MemoryLayer::Persona,
            MemoryLayer::Scenario,
            MemoryLayer::Atomic,
            MemoryLayer::Raw,
        ] {
            let nodes = self.layer(layer);
            if !nodes.is_empty() {
                return nodes;
            }
        }
        Vec::new()
    }

    /// Lossless drill-down: recursively resolve a node back to its full set of
    /// supporting evidence (the transitive closure of `derived_from`).
    pub fn drill_down(&self, id: &Uuid) -> Vec<&LayerNode> {
        let mut result = Vec::new();
        let mut stack = vec![*id];
        let mut seen = std::collections::HashSet::new();
        while let Some(cur) = stack.pop() {
            if !seen.insert(cur) {
                continue;
            }
            if let Some(node) = self.nodes.get(&cur) {
                result.push(node);
                stack.extend(node.derived_from.iter().copied());
            }
        }
        result
    }

    /// Trace a node down to only its L0 ground-truth evidence.
    pub fn evidence(&self, id: &Uuid) -> Vec<&LayerNode> {
        self.drill_down(id)
            .into_iter()
            .filter(|n| n.layer == MemoryLayer::Raw)
            .collect()
    }
}

/// Heuristic builder that ingests a conversation and constructs the L0→L2
/// layers without requiring an LLM. It is deliberately dependency-free so the
/// hierarchy works offline; richer L1/L3 synthesis can be layered on top by
/// supplying LLM-produced facts/personas to the `add_*` methods directly.
#[derive(Debug, Clone)]
pub struct HierarchyBuilder {
    /// Minimum characters for a sentence to qualify as an atomic fact.
    pub min_fact_len: usize,
}

impl Default for HierarchyBuilder {
    fn default() -> Self {
        Self { min_fact_len: 12 }
    }
}

impl HierarchyBuilder {
    /// Build a hierarchy from raw conversation messages. User/assistant turns
    /// become L0 nodes; declarative sentences become L1 atomic facts; facts are
    /// then grouped into an L2 scenario block per builder invocation.
    pub fn build(&self, messages: &[Message]) -> MemoryHierarchy {
        let mut h = MemoryHierarchy::new();
        let mut atomic_ids = Vec::new();

        for msg in messages {
            // Skip system/tool noise for the personalization pyramid.
            if matches!(msg.role, MessageRole::System) {
                continue;
            }
            let raw_id = h.add_raw(msg.content.clone());

            for sentence in split_sentences(&msg.content) {
                let trimmed = sentence.trim();
                if trimmed.len() >= self.min_fact_len && is_declarative(trimmed) {
                    let id = h.add_atomic(trimmed.to_string(), vec![raw_id]);
                    atomic_ids.push(id);
                }
            }
        }

        if !atomic_ids.is_empty() {
            let summary = format!("Scenario block of {} related facts", atomic_ids.len());
            h.add_scenario(summary, atomic_ids);
        }

        h
    }
}

/// Naive sentence splitter on terminal punctuation.
fn split_sentences(text: &str) -> Vec<String> {
    text.split(|c| c == '.' || c == '!' || c == '?' || c == '\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Heuristic for "looks like a stable statement worth remembering" — filters
/// out questions and very short interjections. Because the splitter strips
/// terminal punctuation, interrogatives are detected by their leading word as
/// well as a trailing `?`.
fn is_declarative(sentence: &str) -> bool {
    if sentence.ends_with('?') || sentence.split_whitespace().count() < 3 {
        return false;
    }
    const INTERROGATIVES: &[&str] = &[
        "what", "why", "how", "when", "where", "who", "whom", "whose", "which", "is", "are", "am",
        "was", "were", "do", "does", "did", "can", "could", "would", "should", "will", "shall",
        "may", "might", "have", "has",
    ];
    let first = sentence
        .split_whitespace()
        .next()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .unwrap_or_default();
    !INTERROGATIVES.contains(&first.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: MessageRole, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            timestamp: Utc::now(),
            metadata: None,
        }
    }

    #[test]
    fn builder_creates_layers() {
        let msgs = vec![
            msg(MessageRole::User, "I work at Kalbe as a data engineer. I love coffee."),
            msg(MessageRole::Assistant, "Noted that you enjoy coffee."),
        ];
        let h = HierarchyBuilder::default().build(&msgs);
        assert!(!h.layer(MemoryLayer::Raw).is_empty());
        assert!(!h.layer(MemoryLayer::Atomic).is_empty());
        assert!(!h.layer(MemoryLayer::Scenario).is_empty());
    }

    #[test]
    fn drill_down_reaches_raw_evidence() {
        let mut h = MemoryHierarchy::new();
        let raw = h.add_raw("I work at Kalbe.");
        let fact = h.add_atomic("works at Kalbe", vec![raw]);
        let scenario = h.add_scenario("employment", vec![fact]);
        let persona = h.add_persona("Professional in pharma", vec![scenario]);

        let evidence = h.evidence(&persona);
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].id, raw);
    }

    #[test]
    fn top_context_prefers_highest_layer() {
        let mut h = MemoryHierarchy::new();
        h.add_raw("raw text");
        assert_eq!(h.top_context()[0].layer, MemoryLayer::Raw);
        let r = h.add_raw("more");
        h.add_persona("the persona", vec![r]);
        assert_eq!(h.top_context()[0].layer, MemoryLayer::Persona);
    }

    #[test]
    fn questions_are_not_atomic_facts() {
        let msgs = vec![msg(MessageRole::User, "What is the weather like today?")];
        let h = HierarchyBuilder::default().build(&msgs);
        assert!(h.layer(MemoryLayer::Atomic).is_empty());
    }
}
