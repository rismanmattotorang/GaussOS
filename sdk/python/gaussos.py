"""GaussOS Python SDK — a thin, dependency-light client over the REST API.

Uses only the standard library (urllib), so there is nothing to install.

    from gaussos import GaussOS
    gx = GaussOS("http://localhost:8080")
    gx.create_memory("GaussOS uses an embedded SurrealDB backend",
                     tags=["db"], namespace="docs", quality=0.9)
    print(gx.search("surrealdb"))
    print(gx.hybrid_search("embedded database"))
    gx.ingest_fact("GaussOS", "built_by", "Gaussian Technologies")
    print(gx.graph_search(["GaussOS"]))
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Any, Optional


class GaussOSError(RuntimeError):
    pass


class GaussOS:
    """Client for a running GaussOS server."""

    def __init__(self, base_url: str = "http://localhost:8080", timeout: float = 30.0,
                 api_key: Optional[str] = None) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.api_key = api_key

    # ---- low-level ----
    def _request(self, method: str, path: str, body: Optional[dict] = None) -> Any:
        url = f"{self.base_url}{path}"
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("Content-Type", "application/json")
        if self.api_key:
            req.add_header("Authorization", f"Bearer {self.api_key}")
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw = resp.read()
                return json.loads(raw) if raw else None
        except urllib.error.HTTPError as e:
            raise GaussOSError(f"HTTP {e.code}: {e.read().decode(errors='replace')}") from e
        except urllib.error.URLError as e:
            raise GaussOSError(f"connection error: {e}") from e

    # ---- health / metrics ----
    def health(self) -> Any:
        return self._request("GET", "/health")

    def metrics(self) -> Any:
        return self._request("GET", "/api/v1/metrics")

    def llm_status(self) -> Any:
        return self._request("GET", "/api/v1/llm/status")

    # ---- memories ----
    def create_memory(self, text: str, *, tags: Optional[list[str]] = None,
                      namespace: str = "default", quality: float = 0.5) -> Any:
        return self._request("POST", "/api/v1/memories", {
            "payload": {"Text": text},
            "tags": tags or [],
            "namespace": namespace,
            "quality_score": quality,
        })

    def search(self, text: str, *, namespace: Optional[str] = None,
               payload_type: Optional[str] = None, min_quality: Optional[float] = None,
               limit: int = 50) -> Any:
        body: dict[str, Any] = {"text": text, "limit": limit}
        if namespace:
            body["namespace"] = namespace
        if payload_type:
            body["payload_type"] = payload_type
        if min_quality is not None:
            body["min_quality"] = min_quality
        return self._request("POST", "/api/v1/memories/search", body)

    def hybrid_search(self, text: str, *, embedding: Optional[list[float]] = None,
                      top_k: int = 10) -> Any:
        body: dict[str, Any] = {"text": text, "hybrid": True, "top_k": top_k}
        if embedding is not None:
            body["embedding"] = embedding
        return self._request("POST", "/api/v1/memories/search", body)

    def compare_retrieval(self, text: str, top_k: int = 10) -> Any:
        """White-box: lexical vs vector vs hybrid with score breakdowns."""
        return self._request("POST", "/api/v1/retrieval/compare", {"text": text, "top_k": top_k})

    def delete_memory(self, memory_id: str) -> Any:
        return self._request("DELETE", f"/api/v1/memories/{memory_id}")

    # ---- bi-temporal facts ----
    def ingest_fact(self, subject: str, predicate: str, obj: str) -> Any:
        return self._request("POST", "/api/v1/facts", {
            "subject": subject, "predicate": predicate, "object": obj,
        })

    def facts_about(self, subject: str) -> Any:
        return self._request("GET", f"/api/v1/facts/{subject}")

    def graph_search(self, seeds: list[str]) -> Any:
        """Multi-hop Personalized-PageRank retrieval over the fact graph."""
        return self._request("POST", "/api/v1/facts/graph-search", {"seeds": seeds})

    def fact_graph(self, at: Optional[str] = None) -> Any:
        path = "/api/v1/facts/graph" + (f"?at={at}" if at else "")
        return self._request("GET", path)

    # ---- admin ----
    def export(self) -> Any:
        return self._request("GET", "/api/v1/admin/export")

    def import_memories(self, memories: list[dict]) -> Any:
        return self._request("POST", "/api/v1/admin/import", {"memories": memories})

    def forget(self, namespace: str, *, delete: bool = False) -> Any:
        return self._request("POST", "/api/v1/admin/forget",
                             {"namespace": namespace, "delete_forgotten": delete})


__all__ = ["GaussOS", "GaussOSError"]
