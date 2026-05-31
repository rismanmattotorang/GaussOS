# GaussOS Agent-Memory Research & Implementation Notes

This document records the deep research that informed GaussOS's advanced memory
layer and maps each finding to concrete, implemented Rust capabilities. The goal
is to make GaussOS the most advanced agent-memory engine built in Rust.

Research was conducted across five angles (extraction/consolidation, temporal &
knowledge-graph memory, retrieval algorithms, forgetting & organization, and
evaluation benchmarks), cross-validating claims across primary papers, official
docs, and engineering write-ups. Numbers below are from those sources; vendor
self-reported benchmark figures are flagged as such.

## What the research said wins benchmarks

- **Temporal modeling is the single biggest differentiator.** On LoCoMo and
  LongMemEval, systems with explicit temporal handling score 55–85% on temporal
  questions vs ~21–23% for naive memory. Zep's temporal knowledge graph reports
  +18.5% (GPT-4o) over a full-context baseline on LongMemEval with ~90% latency
  reduction. *(Zep, arXiv 2501.13956 — vendor self-reported.)*
- **Multi-hop retrieval needs a graph walk.** HippoRAG's Personalized PageRank
  over a knowledge graph lifts 2Wiki Recall@5 from 76.5% → 90.4% and MuSiQue
  Recall@5 69.7% → 74.7% vs a strong dense baseline. *(HippoRAG 2, arXiv 2502.14802.)*
- **Hybrid retrieval + RRF + reranking** are the consistently cited gain drivers;
  RRF with k≈60 fuses BM25 + dense without score-scale issues.
- **Scale needs ANN + quantization.** HNSW reaches recall@10 ≈ 0.95 with M≈32,
  ef 64–128; binary quantization gives ~32× memory reduction (Hamming/popcount),
  scalar int8 ~4× with ~1–2% recall loss.
- **Forgetting & importance.** Generative Agents rank by
  `recency(0.995^hours) + importance + relevance`, each min-max normalized;
  MemoryBank applies an Ebbinghaus retention curve with reinforcement on access.
- **The hardest open problem (MemoryAgentBench, arXiv 2507.05257):** *all*
  current systems fail at multi-hop conflict resolution (≤6%). GaussOS targets
  this directly with bi-temporal supersession + PPR multi-hop retrieval.

## What was already in GaussOS (and how it aligns)

| Research technique | GaussOS module | Status |
|---|---|---|
| Bi-temporal graph (Zep's 4 timestamps, supersede-not-delete) | `memory/temporal.rs` | ✅ already implemented; matches Zep's valid/transaction-time model |
| Hybrid BM25 + dense + RRF + MMR | `memory/retrieval.rs` | ✅ |
| Ebbinghaus forgetting curve + reinforcement | `memory/decay.rs` | ✅ |
| L0→L3 progressive disclosure | `memory/hierarchy.rs` | ✅ |
| Episodic / semantic / procedural types (CoALA) | `core::MemoryPayload` | ✅ |

## What this round added (research-driven, all pure-Rust, fully tested)

### 1. HNSW approximate-nearest-neighbour index — `memory/ann/hnsw.rs`
A from-scratch Hierarchical Navigable Small World graph (Malkov & Yashunin):
multi-layer navigable graph, heuristic neighbour selection (Algorithm 4),
greedy descent + ef-bounded layer search. Defaults `m=16, ef_construction=200,
ef_search=64`. Verified recall@5 ≥ 0.9 vs brute force in tests. Replaces the
O(N) brute-force re-rank with sublinear search. Wired into `MemoryManager`:
embeddings are indexed on `create_memory`/`create_memories_batch`, queryable via
`ann_search` and the `POST /memories/ann-search` endpoint.

### 2. Vector quantization — `memory/ann/quantization.rs`
- **Binary quantization**: 1 sign bit/dim packed into `u64`, Hamming distance via
  `count_ones()` (popcount) → ~32× smaller, ideal as a coarse pre-filter.
- **Scalar (int8) quantization**: affine min/max mapping → ~4× smaller with a
  recoverable ~1–2% recall loss.

### 3. Personalized PageRank multi-hop retrieval — `memory/graph_retrieval.rs`
HippoRAG-style retriever over the bi-temporal fact graph. Builds an entity graph
from live facts, seeds a restart distribution at the query entities, runs PPR by
power iteration `r = (1-α)·s + α·Wᵀr`, and scores facts by the
specificity-weighted (IDF-like) PageRank mass of their endpoints. Surfaces
multi-hop evidence flat similarity misses. Wired via `MemoryManager::graph_search`
and `POST /facts/graph-search`.

### 4. Generative-Agents retrieval scorer — `memory/scoring.rs`
The exact `recency + importance + relevance` recipe (weights 1.0, recency decay
0.995/hour), with each component min-max normalized across the candidate set.
Complements the RRF path (which fuses rankings) by fusing normalized scores —
the better fit for conversational/companion agents where recency and importance
matter as much as semantic relevance. Wired via
`MemoryManager::generative_agent_search`.

## Why GaussOS is now ahead

GaussOS combines, in one compiled, type-safe Rust engine:
- Zep's bi-temporal graph **and** HippoRAG's PPR multi-hop walk (most systems
  have one or the other),
- a real HNSW index + quantization for scale (most memory frameworks delegate
  this to an external vector DB),
- both ranking philosophies — RRF rank-fusion **and** the Generative-Agents
  normalized-score blend — selectable per workload,
- a cognitive forgetting curve and L0→L3 progressive disclosure,

directly targeting the benchmark frontier (temporal + multi-hop + conflict
resolution) that independent evaluations show remains unsolved.

## Primary sources

- Mem0 — arXiv 2504.19413
- MemGPT / Letta — arXiv 2310.08560; letta.com (sleep-time compute)
- Generative Agents — arXiv 2304.03442
- Zep / Graphiti — arXiv 2501.13956
- A-Mem — arXiv 2502.12110
- HippoRAG / HippoRAG 2 — arXiv 2405.14831 / 2502.14802
- GraphRAG — arXiv 2404.16130
- MemoryBank — arXiv 2305.10250
- CoALA — arXiv 2309.02427
- LongMemEval — arXiv 2410.10813
- MemoryAgentBench — arXiv 2507.05257
- HNSW — Malkov & Yashunin, "Efficient and robust ANN search using HNSW graphs"

*Caveat: most published agent-memory leaderboard numbers are vendor self-reported
with LLM-as-judge scoring and have documented reproduction disputes; treat
cross-system "X beats Y" claims with skepticism. The implemented techniques were
chosen for sound algorithmic grounding, not leaderboard marketing.*
