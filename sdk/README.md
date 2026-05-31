# GaussOS SDKs

Thin, dependency-light clients over the GaussOS REST API. They make it trivial
to migrate from Mem0/Zep/Letta — point them at a running GaussOS server.

## Python (`python/gaussos.py`)

Standard-library only (no `pip install`).

```python
from gaussos import GaussOS

gx = GaussOS("http://localhost:8080")
gx.create_memory("GaussOS uses an embedded SurrealDB backend", tags=["db"], namespace="docs", quality=0.9)
print(gx.search("surrealdb"))
print(gx.compare_retrieval("embedded database"))   # white-box BM25 vs vector vs hybrid
gx.ingest_fact("GaussOS", "built_by", "Gaussian Technologies")
print(gx.graph_search(["GaussOS"]))                # multi-hop Personalized PageRank
```

## TypeScript (`typescript/gaussos.ts`)

Uses the global `fetch` — runs in Deno, Node 18+, and browsers.

```ts
import { GaussOS } from "./gaussos.ts";

const gx = new GaussOS("http://localhost:8080");
await gx.createMemory("GaussOS uses an embedded SurrealDB backend", { tags: ["db"], namespace: "docs", quality: 0.9 });
console.log(await gx.search("surrealdb"));
console.log(await gx.hybridSearch("embedded database"));
await gx.ingestFact("GaussOS", "built_by", "Gaussian Technologies");
console.log(await gx.graphSearch(["GaussOS"]));
```

Both clients cover: memories (create/search/hybrid/compare/delete), bi-temporal
facts (ingest/about/graph/graph-search), snapshot export/import, the forgetting
pass, and health/metrics/LLM status. See [`../README.md`](../README.md) for the
full API and provider configuration.
