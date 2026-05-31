# GaussOS Agent Memory — The Complete Memory Engine in Rust

GaussOS aims to be **the most complete agent-memory system written in Rust**.
This document describes the intelligence layer that sets it apart, and how each
capability was informed by studying the leading agent-memory systems —
**TencentDB-Agent-Memory**, **Zep/Graphiti**, **Letta/MemGPT**, and **Mem0** —
and then implemented natively in safe, fast Rust.

## Why a new layer?

The base GaussOS already provided enterprise plumbing: hybrid storage
(PostgreSQL + SurrealDB + Redis), a graph engine, tiered L1/L2/L3 caches,
SIMD-accelerated vector ops, auth, and a TUI. What it lacked were the
*cognitive* primitives that modern agent-memory research has converged on.
Those primitives now live in [`src/memory`](src/memory) as four cohesive,
dependency-light, fully unit-tested modules.

| Capability | GaussOS module | Inspired by | What it gives an agent |
|---|---|---|---|
| Hybrid retrieval (BM25 + vector + RRF + MMR) | [`retrieval.rs`](src/memory/retrieval.rs) | TencentDB RRF hybrid search | High-recall, low-redundancy context |
| Bi-temporal knowledge graph | [`temporal.rs`](src/memory/temporal.rs) | Zep / Graphiti | Corrections without data loss; point-in-time recall |
| Forgetting curve & salience | [`decay.rs`](src/memory/decay.rs) | Cognitive science + Letta sleep-time | Bounded, self-pruning memory |
| Hierarchical L0→L3 pyramid | [`hierarchy.rs`](src/memory/hierarchy.rs) | TencentDB progressive disclosure | Cheap top-layer context with lossless drill-down |

---

## 1. Hybrid Retrieval Engine (`retrieval.rs`)

Pure-keyword search misses paraphrases; pure-vector search misses exact terms,
IDs, and rare tokens. GaussOS fuses both with **Reciprocal Rank Fusion (RRF)** —
the same strategy that drives TencentDB-Agent-Memory's recall gains — and then
applies two re-ranking passes the competitors only partially offer:

- **BM25** lexical scoring with precomputed corpus statistics (IDF, avg doc len).
- **Dense vector** cosine similarity over payload embeddings.
- **RRF fusion**: `score = Σ wᵢ / (k + rankᵢ)` — robust to incomparable score scales.
- **MMR re-ranking**: maximises `λ·relevance − (1−λ)·redundancy` for diverse context.
- **Temporal decay + salience boost**: ties break toward recent, important memories.

Every result is returned as a `ScoredMemory` with a full breakdown
(`bm25_score`, `vector_score`, `recency_score`, ranks) — *white-box
debuggability*, another TencentDB design principle.

```rust
let retriever = HybridRetriever::new(candidates, HybridSearchConfig::default());
let results = retriever.search("rust memory", Some(&query_embedding));
```

## 2. Bi-Temporal Knowledge (`temporal.rs`)

Real agents must handle *"I changed jobs"* without erasing history. Following
Zep/Graphiti's bi-temporal model, every `TemporalFact` tracks **two time axes**:

- **Valid time** (`valid_at` / `invalid_at`) — when the fact was true in the world.
- **Transaction time** (`recorded_at` / `expired_at`) — when the system knew it.

Conflicting assertions are **superseded, not deleted** — the prior record is
marked invalid/expired and kept for audit. This unlocks point-in-time queries:

```rust
store.ingest(TemporalFact::new("user:edwin", "employer", "Kalbe")); // supersedes prior
store.current_value("user:edwin", "employer");   // -> [Kalbe]
store.history("user:edwin", "employer");          // -> [Kalbe, OldCorp] (audit trail)
store.facts_valid_at(last_tuesday);               // point-in-time recall
```

## 3. Forgetting Curve & Salience (`decay.rs`)

Unbounded memory growth kills retrieval quality and inflates cost. GaussOS
applies the **Ebbinghaus forgetting curve** with reinforcement — the kind of
"sleep-time" housekeeping Letta performs during idle periods:

- **Recency**: exponential decay since last access.
- **Frequency**: log-scaled access count (the spacing effect).
- **Importance**: intrinsic quality blended with declared `Priority`.
- **Reinforcement**: repeated access flattens the decay curve.

`ForgettingCurve::classify` partitions memories into **Retain / Archive /
Forget** buckets, and `Priority::Critical` memories are pinned forever.

## 4. Hierarchical L0→L3 Pyramid (`hierarchy.rs`)

TencentDB's headline result — up to 61% token reduction — comes from
**progressive disclosure**: keep abstract layers in context, drill down to
evidence only when needed. GaussOS models the same pyramid:

- **L0 Raw** — verbatim messages / tool outputs.
- **L1 Atomic** — discrete facts distilled from L0.
- **L2 Scenario** — thematic aggregations of facts.
- **L3 Persona** — synthesised, stable user profile.

Each node links to the nodes it was derived from, so `drill_down`/`evidence`
provide **lossless** traceability back to ground truth — abstraction you can
always reverse.

---

## Competitive positioning

- **vs TencentDB-Agent-Memory** — GaussOS matches its RRF hybrid retrieval and
  L0→L3 progressive disclosure, and adds a bi-temporal graph and a forgetting
  model it does not have, all in compiled Rust rather than Python.
- **vs Zep/Graphiti** — matches the bi-temporal fact model and adds
  cognitive forgetting + a fused lexical/semantic retriever and MMR diversity.
- **vs Letta/MemGPT** — matches tiered retention/sleep-time consolidation
  (retain/archive/forget) and adds bi-temporal history and hierarchical recall.
- **vs Mem0** — matches extract-and-update semantics with explicit, auditable
  supersession instead of opaque overwrites.

All four modules are storage-agnostic, allocation-light, and covered by unit
tests (`cargo test`) plus an end-to-end example
(`cargo run --example agent_memory_showcase`).

## Agent LLM layer (multi-provider)

The agent layer that turns memory into action is **provider-agnostic**. A single
flexible client ([`src/agents/llm.rs`](src/agents/llm.rs)) speaks both the
Anthropic Messages API and the OpenAI-compatible Chat Completions API, with
presets for **OpenAI (GPT), DeepSeek, Qwen (DashScope), BytePlus (ModelArk),
OpenRouter, and Anthropic (Claude)**, plus a `custom` mode for any
OpenAI-compatible endpoint (Ollama, vLLM, LM Studio, gateways).

Selection and configuration are entirely environment-driven (`LLM_PROVIDER`,
`LLM_MODEL`, `LLM_BASE_URL`, `LLM_API_KEY`, or provider-specific key envs); see
the [LLM Providers section of the README](README.md#-llm-providers-flexible-multi-vendor)
and [`.env.example`](.env.example). When no key is configured the agent layer
degrades honestly (`llm_not_configured`) rather than fabricating output.
