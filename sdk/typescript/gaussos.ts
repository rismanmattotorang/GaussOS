// GaussOS TypeScript SDK — a thin client over the REST API.
// Works in Deno, Node 18+, and browsers (uses the global `fetch`).
//
//   import { GaussOS } from "./gaussos.ts";
//   const gx = new GaussOS("http://localhost:8080");
//   await gx.createMemory("GaussOS uses an embedded SurrealDB backend",
//                         { tags: ["db"], namespace: "docs", quality: 0.9 });
//   console.log(await gx.search("surrealdb"));
//   console.log(await gx.hybridSearch("embedded database"));
//   await gx.ingestFact("GaussOS", "built_by", "Gaussian Technologies");
//   console.log(await gx.graphSearch(["GaussOS"]));

export interface CreateMemoryOpts {
  tags?: string[];
  namespace?: string;
  quality?: number;
}

export interface SearchOpts {
  namespace?: string;
  payloadType?: string;
  minQuality?: number;
  limit?: number;
}

export class GaussOSError extends Error {}

export class GaussOS {
  constructor(
    private baseUrl: string = "http://localhost:8080",
    private apiKey?: string,
  ) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
  }

  private async request<T = unknown>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (this.apiKey) headers["Authorization"] = `Bearer ${this.apiKey}`;
    const res = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    if (!res.ok) {
      throw new GaussOSError(`HTTP ${res.status}: ${await res.text()}`);
    }
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }

  // ---- health / metrics ----
  health(): Promise<unknown> {
    return this.request("GET", "/health");
  }
  metrics(): Promise<unknown> {
    return this.request("GET", "/api/v1/metrics");
  }
  llmStatus(): Promise<unknown> {
    return this.request("GET", "/api/v1/llm/status");
  }

  // ---- memories ----
  createMemory(text: string, opts: CreateMemoryOpts = {}): Promise<unknown> {
    return this.request("POST", "/api/v1/memories", {
      payload: { Text: text },
      tags: opts.tags ?? [],
      namespace: opts.namespace ?? "default",
      quality_score: opts.quality ?? 0.5,
    });
  }

  search(text: string, opts: SearchOpts = {}): Promise<unknown> {
    const body: Record<string, unknown> = { text, limit: opts.limit ?? 50 };
    if (opts.namespace) body.namespace = opts.namespace;
    if (opts.payloadType) body.payload_type = opts.payloadType;
    if (opts.minQuality !== undefined) body.min_quality = opts.minQuality;
    return this.request("POST", "/api/v1/memories/search", body);
  }

  hybridSearch(text: string, embedding?: number[], topK = 10): Promise<unknown> {
    const body: Record<string, unknown> = { text, hybrid: true, top_k: topK };
    if (embedding) body.embedding = embedding;
    return this.request("POST", "/api/v1/memories/search", body);
  }

  /** White-box: lexical vs vector vs hybrid with score breakdowns. */
  compareRetrieval(text: string, topK = 10): Promise<unknown> {
    return this.request("POST", "/api/v1/retrieval/compare", { text, top_k: topK });
  }

  deleteMemory(id: string): Promise<unknown> {
    return this.request("DELETE", `/api/v1/memories/${id}`);
  }

  // ---- bi-temporal facts ----
  ingestFact(subject: string, predicate: string, object: string): Promise<unknown> {
    return this.request("POST", "/api/v1/facts", { subject, predicate, object });
  }
  factsAbout(subject: string): Promise<unknown> {
    return this.request("GET", `/api/v1/facts/${subject}`);
  }
  /** Multi-hop Personalized-PageRank retrieval over the fact graph. */
  graphSearch(seeds: string[]): Promise<unknown> {
    return this.request("POST", "/api/v1/facts/graph-search", { seeds });
  }
  factGraph(at?: string): Promise<unknown> {
    return this.request("GET", "/api/v1/facts/graph" + (at ? `?at=${encodeURIComponent(at)}` : ""));
  }

  // ---- admin ----
  export(): Promise<unknown> {
    return this.request("GET", "/api/v1/admin/export");
  }
  importMemories(memories: unknown[]): Promise<unknown> {
    return this.request("POST", "/api/v1/admin/import", { memories });
  }
  forget(namespace: string, deleteForgotten = false): Promise<unknown> {
    return this.request("POST", "/api/v1/admin/forget", {
      namespace,
      delete_forgotten: deleteForgotten,
    });
  }
}
