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
12. **✅ Auto‑vectorising distance kernels** (`memory/ann/distance.rs`) for
    dot/cosine/L2/Hamming — 8‑lane independent accumulators the compiler lowers
    to packed SIMD (AVX/AVX‑512 with `target‑cpu=native`); no unsafe, portable.
    🧭 Remaining: arena allocation for `MemCube` payloads.
13. **✅ Sharded vector index** (`memory/ann/sharded.rs`): `ShardedHnsw`
    partitions vectors across N per‑shard‑locked HNSW graphs so writers to
    different shards don't contend; queries fan out and merge a global top‑k.
14. **✅ Columnar episodic store** (`memory/episodic.rs`): time‑sorted columns
    with O(log n) binary‑search range queries, recent/namespace views, and
    retention pruning for the L0 layer.

> Remaining Phase 2 (tracked): HNSW incremental compaction is done; mmap /
> DiskANN on‑disk tier and `MemCube` arena allocation are deferred to a future
> performance pass.

## Phase 3 — Web UI / UX (elegant, professional, real‑time)

15. **✅ Design‑system pass.** ⌘K command palette (navigate + quick actions);
    dark/light/**system** theme cycle applied on load (`data-theme`) with a light
    palette; `prefers-reduced-motion` honoured (WCAG). *Files:* `app.js`, `styles.css`.
16. **✅ Memory Explorer.** Faceted **live** search (full‑text + namespace +
    payload type + min‑quality) → `POST /api/v1/memories/search`, real results
    table (type/content/namespace/quality) + inline delete. The bi‑temporal
    "as‑of" view ships in the Knowledge Graph page (#17).
17. **✅ Knowledge‑graph viewer.** Canvas entity‑graph with degree‑sized nodes, a
    **bi‑temporal "as‑of" time slider** (`GET /api/v1/facts/graph?at=`), and
    click‑to‑trace **Personalized PageRank** highlighting. Backed by
    `MemoryManager::fact_graph`. Verified live (current + time‑travel views).
18. **✅ Retrieval Playground.** Side‑by‑side **lexical (BM25) vs vector vs hybrid
    (RRF)** results with the full score breakdown (per‑result bm25/vector/recency
    scores and ranks) — a "why this result" white‑box panel no competitor offers.
    Backed by `MemoryManager::compare_retrieval` and `POST /api/v1/retrieval/compare`;
    served as a Web UI page (`web-ui` nav → *Retrieval Playground*). Verified live.
    🧭 Remaining: add a PPR column.
19. **🟡 Live ops dashboard.** ✅ Real SSE charts; `vector_index_size` + `facts`
    in `/api/v1/metrics`; a **forgetting‑pass control + outcome panel**
    (retained/archived/forgotten) in Settings via `POST /api/v1/admin/forget`.
    🧭 Remaining: per‑provider LLM latency/cost tracking.
20. **✅ First‑run wizard.** On first load, shows the active LLM provider/model
    and configured state (via `GET /api/v1/llm/status`), offers one‑click
    "Seed sample memories", and remembers completion. Verified live.

## Phase 4 — TUI (a real operator cockpit)

21. **✅ Real, data-driven views.** Replaced the placeholder Query/Config/Graphs
    screens (which showed fabricated data) with live, server-backed views in
    `tui/app.rs`. *Files:* `src/tui/app.rs`.
22. **✅ Query REPL.** An interactive search REPL: type on the Query tab (keys go
    to the input, Tab leaves), press Enter to run a live `/api/v1/memories/search`,
    and see real results (type · quality · content). The Help screen documents it.
23. **✅ Real ops views + controls.** Knowledge Graph tab: live entity/relation
    counts + degree bar-chart (`GET /api/v1/facts/graph`). Config tab: live
    server URL, connection, LLM provider/model/status, build features. **`/`
    vim-style quick-filter** for the Memory Browser (live, case-insensitive;
    title shows the query + match count). **`F` in-TUI forgetting pass** on the
    selected namespace (`POST /api/v1/admin/forget`, result in the status bar).
    **Alerts** (offline / high CPU / low cache hit) surfaced in the footer.

> Implemented intuitively: the Query tab is a focused input with examples and a
> visible cursor; `/` filters with an obvious title; the footer always shows the
> key legend (Palette · Filter · Forget · Help · Quit) and the top alert; Help
> (`?`) documents every key. The `tui/*.rs` stub modules remain as future homes
> for extracting these views from `app.rs`.

## Phase 5 — Platform & operations

24. **✅ Snapshot backup/restore + persistence.** `GET /api/v1/admin/export`
    (full JSON snapshot) and `POST /api/v1/admin/import` (restore/migrate), via
    `MemoryManager::export_all`/`import_memories`; verified round-trip through the
    Python SDK. ✅ **RocksDB on-disk persistence**: set `GAUSSOS_SURREAL_PATH`
    (or a `rocksdb://`/`file://` endpoint) and the embedded SurrealDB persists to
    disk instead of memory (`src/database/surreal.rs`). 🧭 Remaining: external
    SurrealDB/Postgres for HA.
25. **✅ Distributed mode.** Consistent-hash ring with virtual nodes
    (`src/database/cluster.rs`): `HashRing` + `ShardRouter` route keys to a
    primary + replicas, rebalancing only ~`1/N` of keys when a node joins/leaves
    (proven by tests). 🧭 Remaining: the networking/replication transport that
    consumes the router.
26. **✅ Observability.** Prometheus text-exposition endpoint at
    `/metrics/prometheus` (memories, storage, vector index, facts, CPU, memory),
    plus an **OpenTelemetry/OTLP trace exporter** (`src/observability.rs`,
    `otlp_traces_json` + background flusher) that activates on the standard
    `OTEL_EXPORTER_OTLP_ENDPOINT`, and **Grafana dashboards + Prometheus config**
    under `deploy/`. 🧭 Remaining: wire lifecycle/scheduler spans end-to-end.
27. **✅ Deploy.** Multi-stage `Dockerfile` (slim runtime, non-root, healthcheck)
    + `.dockerignore` + `docker-compose.yml` (API + Deno web UI + opt-in
    Prometheus/Grafana `observability` profile) + a **Helm chart**
    (`deploy/helm/gaussos/`: Deployment, Service, PVC for the RocksDB store, HPA,
    ServiceMonitor).
28. **✅ SDKs.** Thin **Python** (`sdk/python/gaussos.py`, stdlib-only) and
    **TypeScript** (`sdk/typescript/gaussos.ts`, fetch-based) clients covering
    memories, hybrid/compare retrieval, bi-temporal facts, graph search,
    export/import, and forgetting — so Mem0/Zep/Letta users can migrate easily.
    Python compiles; TS type-checks under Deno.
29. **✅ Security & compliance.** `src/security/`: tamper-evident **audit log**
    (rolling hash chain, `audit.rs`), **per-namespace RLS** (default-deny, role +
    wildcard subjects, descendant-covering grants, explicit-deny-wins, `rls.rs`),
    and feature-gated **field-level AES-256-GCM encryption with key rotation**
    (`encryption.rs`, `--features encryption`: per-message random nonce, decrypt
    any historical key version, encrypt with the current one).

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
