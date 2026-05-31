# GaussOS Roadmap — The Plan to Be the Superior Agent Memory

*By Gaussian Technologies — an Indonesian deep‑tech startup.*

This is the concrete plan to extend GaussOS's lead (see [`BENCHMARK.md`](BENCHMARK.md))
over TencentDB, Zep, Letta, and Mem0 across **algorithms, data structures, Web
UI/UX, TUI, and operations**. Items are ordered by leverage. Each lists the
**goal**, the **approach**, and the **target file(s)** so it is executable, not
aspirational.

Status legend: ✅ done · 🟡 partial · 🧭 planned.

---

## Phase 0 — Prove it (credibility)

1. **🧭 LoCoMo + LongMemEval harness.** Run the public benchmarks end‑to‑end with
   the multi‑provider LLM client and publish reproducible scripts + scores
   (not vendor‑style cherry‑picks). *Approach:* `benches/agent_memory_bench/`
   driver that ingests the dataset through the real API and scores with an
   LLM‑judge whose prompts are committed. *Why first:* turns capability claims
   into measured claims.
2. **🧭 Reproducible perf benchmarks.** Criterion suites for BM25, HNSW
   (recall@k vs latency vs `ef`), PPR convergence, and end‑to‑end query latency;
   publish a `BENCHMARKS_RESULTS.md` with hardware + methodology.

## Phase 1 — Algorithms (depth that wins benchmarks)

3. **🟡→✅ LLM‑driven extract→update ingestion** (Mem0/Zep‑class). Wire the
   multi‑provider client into ingestion: extract atomic facts from turns, embed,
   retrieve neighbours, and have the LLM emit ADD/UPDATE/DELETE/NOOP, executed
   against the bi‑temporal store (which already supersedes correctly).
   *Files:* `memory/extraction.rs`, `memory/temporal.rs`, `agents/llm.rs`.
4. **🧭 Temporal extraction.** Resolve absolute *and* relative timestamps
   ("two weeks ago") into `valid_at`, closing Zep's last temporal edge.
   *Files:* `memory/temporal.rs` + an extraction prompt.
5. **🧭 Community detection + hierarchical summaries** (GraphRAG‑class). Leiden /
   label‑propagation over the entity graph, with LLM community summaries feeding
   the L2/L3 hierarchy and global "sensemaking" queries.
   *Files:* new `memory/community.rs`, integrates `graph_retrieval.rs` + `hierarchy.rs`.
6. **🧭 Cross‑encoder / late‑interaction rerank.** Optional ONNX reranker (and a
   ColBERT‑style MaxSim path) after RRF for top‑k precision.
   *Files:* new `memory/rerank.rs` (feature‑gated, `ort`/`candle`).
7. **🧭 Self‑editing memory blocks + autonomous sleep‑time agent** (Letta‑class).
   First‑class editable core‑memory blocks; a background "sleep‑time" task that
   consolidates, reflects (Generative‑Agents reflection), and rewrites blocks
   during idle — built on the existing forgetting/consolidation pass.
   *Files:* new `memory/blocks.rs`, `agents/sleeptime.rs`.
8. **🧭 Symbolic task canvas** (TencentDB‑class). Render task/episode state as
   high‑density Mermaid for token‑efficient, white‑box context.
   *Files:* new `memory/canvas.rs`.
9. **🧭 Query rewriting / HyDE.** LLM query expansion for recall, scored against
   the *original* query at rerank time.

## Phase 2 — Data structures & performance

10. **🟡 HNSW: deletes, persistence, on‑disk tier.** ✅ Tombstoned deletes (with
    reactivation on re‑insert) and ✅ byte‑buffer persistence
    (`to_bytes`/`from_bytes`) are done and tested; `delete_memory` now
    soft‑deletes from the index, and `MemoryManager` can export/import it.
    🧭 Remaining: incremental compaction, mmap‑backed persistence, and an
    optional DiskANN/Vamana on‑disk graph for billion‑scale.
    *Files:* `memory/ann/hnsw.rs`, `memory/manager.rs`, future `ann/disk.rs`.
11. **✅ Quantized ANN search path.** `memory/ann/quantized_index.rs` implements
    binary pre‑filter (Hamming/popcount) → int8 rescore ("oversample +
    rescore"): ~3.5× smaller than f32 with high recall (tested vs brute force).
12. **🧭 SIMD/AVX‑512 distance kernels** for cosine/L2/Hamming, behind `simd`.
13. **🧭 Lock‑free, sharded vector index** for concurrent insert/search; arena
    allocation for `MemCube` payloads to cut allocator pressure.
14. **🧭 Columnar episodic store** for fast time‑range scans of the L0 layer.

## Phase 3 — Web UI / UX (elegant, professional, real‑time)

15. **🧭 Design‑system pass.** Build on the existing "Cosmic Minimalism" tokens:
    refine typography scale, motion (reduced‑motion aware), light/dark/system,
    accessible contrast (WCAG AA), and a command palette (⌘K).
    *Files:* `web-ui/static/styles.css`, `app.js`.
16. **🧭 Memory Explorer.** Faceted search (namespace/tags/quality/date/type),
    bi‑temporal timeline view ("as of" slider), and per‑memory provenance &
    relationship inspector — surfacing GaussOS's unique temporal model visually.
17. **🧭 Knowledge‑graph viewer.** Interactive entity graph (WebGL/canvas) with
    PPR result highlighting and multi‑hop path tracing.
18. **🧭 Retrieval Playground.** Side‑by‑side BM25 vs vector vs hybrid (RRF) vs
    PPR results with the score breakdown GaussOS already returns — a "why this
    result" white‑box panel no competitor offers.
19. **🧭 Live ops dashboard.** Real SSE charts (done) + HNSW/index health,
    forgetting‑pass outcomes, and per‑provider LLM latency/cost.
20. **🧭 First‑run wizard.** Pick LLM provider, paste key, seed sample memories,
    and land in a working dashboard in under a minute.

## Phase 4 — TUI (a real operator cockpit)

21. **🧭 Flesh out the stub views** (`dashboard`, `memory_browser`,
    `query_repl`, `agent_manager`, `log_viewer`, `config_editor`) on the solid
    `tui/app.rs` shell — currently 5‑line placeholders.
    *Files:* `src/tui/*.rs`.
22. **🧭 Memory browser + query REPL** with vim‑style keys, fuzzy find, and the
    same score‑breakdown panel as the Web UI.
23. **🧭 Unicode charts** for live metrics, an alerts pane, and a forgetting/
    consolidation control panel — all driven by the real API.

## Phase 5 — Platform & operations

24. **🧭 Persistence for SurrealDB embedded** (RocksDB path) with snapshot
    backup/restore; pluggable external SurrealDB/Postgres for HA.
25. **🧭 Distributed mode.** Sharded namespaces, replication, and a consistent
    hashing ring for horizontal scale.
26. **🧭 Observability.** OpenTelemetry traces, Prometheus metrics, Grafana
    dashboards; wire the lifecycle/scheduler fully into the server runtime.
27. **🧭 Deploy.** Distroless Docker image, Helm chart, and a one‑command
    `docker compose` (GaussOS + optional Postgres/SurrealDB).
28. **🧭 SDKs.** Thin Python and TypeScript clients over the REST/streaming API
    so existing Mem0/Zep/Letta users can migrate with minimal changes.
29. **🧭 Security & compliance.** Field‑level encryption, audit log, key
    rotation, and per‑namespace RLS.

---

## Guiding principles

- **Honesty over hype.** Capability claims map to code; accuracy claims wait for
  reproducible benchmarks.
- **White‑box by default.** Every ranking decision is inspectable (the score
  breakdowns already shipped) — a structural advantage over opaque vector stores.
- **One fast binary.** Stay runnable fully offline; make external services
  optional, never required.
- **Rust all the way down.** Memory safety, no GC pauses, fearless concurrency.

GaussOS is built by **Gaussian Technologies**, an Indonesian deep‑tech startup,
to be the most complete and trustworthy agent‑memory engine — open, fast, and
correct.
