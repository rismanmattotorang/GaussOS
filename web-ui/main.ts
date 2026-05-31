// GaussOS v3.0 - Premium Web UI
// A distinctive, modern interface built with Cosmic Minimalism design
// Enhanced with real-time updates and comprehensive error handling

// Uses Deno's built-in HTTP server (no external std dependency required).

const PORT = parseInt(Deno.env.get("PORT") || "3000");
const BACKEND_URL = Deno.env.get("BACKEND_URL") || "http://localhost:8080";

// Connection pool for backend
const backendConnections = new Map<string, { lastUsed: number; healthy: boolean }>();

// Retry configuration
const RETRY_CONFIG = { maxRetries: 3, initialDelay: 100, maxDelay: 2000 };

// Content type mapping
function getContentType(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase();
    const types: Record<string, string> = {
        'css': 'text/css; charset=utf-8',
        'js': 'application/javascript; charset=utf-8',
        'json': 'application/json; charset=utf-8',
        'html': 'text/html; charset=utf-8',
        'png': 'image/png',
        'svg': 'image/svg+xml',
        'woff2': 'font/woff2',
    };
    return types[ext || ''] || 'application/octet-stream';
}

// Request handler
async function handler(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;
    
    // Static files
    if (path.startsWith("/static/")) {
        try {
            const filePath = `${Deno.cwd()}/static${path.replace("/static", "")}`;
            const content = await Deno.readTextFile(filePath);
            return new Response(content, {
                headers: {
                    "Content-Type": getContentType(filePath),
                    "Cache-Control": "public, max-age=86400",
                },
            });
        } catch {
            return new Response("Not found", { status: 404 });
        }
    }
    
    // API endpoints - proxy to backend
    if (path.startsWith("/api/")) {
        return handleApi(req, path);
    }
    
    // SSE endpoints
    if (path === "/events/metrics") {
        return handleMetricsStream();
    }
    
    // Serve main app
    return new Response(createHtml(), {
        headers: {
            "Content-Type": "text/html; charset=utf-8",
            "Cache-Control": "no-store",
        },
    });
}

// API handler with backend proxy and retry logic
async function handleApi(req: Request, path: string): Promise<Response> {
    const backendUrl = `${BACKEND_URL}${path}`;
    let lastError: Error | null = null;
    
    for (let attempt = 0; attempt < RETRY_CONFIG.maxRetries; attempt++) {
        try {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 10000);
            
            const reqBody = req.method !== "GET" && req.method !== "HEAD" ? await req.text() : null;
            const response = await fetch(backendUrl, {
                method: req.method,
                headers: {
                    "Content-Type": "application/json",
                    "X-Request-ID": crypto.randomUUID(),
                    "X-Forwarded-For": req.headers.get("x-forwarded-for") || "unknown",
                },
                body: reqBody,
                signal: controller.signal,
            });
            
            clearTimeout(timeoutId);
            
            const responseHeaders = new Headers({
                "Content-Type": "application/json",
                "X-Backend-Status": response.status.toString(),
            });
            
            // Copy CORS headers
            if (response.headers.has("access-control-allow-origin")) {
                responseHeaders.set("access-control-allow-origin", response.headers.get("access-control-allow-origin")!);
            }
            
            return new Response(await response.text(), {
                status: response.status,
                headers: responseHeaders,
            });
        } catch (error) {
            lastError = error instanceof Error ? error : new Error(String(error));
            
            if (attempt < RETRY_CONFIG.maxRetries - 1) {
                const delay = Math.min(RETRY_CONFIG.initialDelay * Math.pow(2, attempt), RETRY_CONFIG.maxDelay);
                await new Promise(resolve => setTimeout(resolve, delay));
            }
        }
    }
    
    // Backend unreachable: report it honestly rather than fabricating data.
    // (Fake metrics/memories would mislead anyone evaluating the engine.)
    console.warn(`Backend unavailable after ${RETRY_CONFIG.maxRetries} attempts:`, lastError?.message);
    return new Response(
        JSON.stringify({ error: "backend_unavailable", detail: lastError?.message ?? "unknown" }),
        { status: 503, headers: { "Content-Type": "application/json", "X-Backend-Status": "unavailable" } },
    );
}

// Metrics SSE stream — polls the backend for REAL metrics and forwards them.
// (Previously this emitted random numbers, so the live dashboard showed fake
// data regardless of the backend.)
function handleMetricsStream(): Response {
    let intervalId: number | undefined;

    const body = new ReadableStream({
        start(controller) {
            const encoder = new TextEncoder();

            const sendMetrics = async () => {
                let data: Record<string, unknown>;
                try {
                    const res = await fetch(`${BACKEND_URL}/api/v1/metrics`, {
                        signal: AbortSignal.timeout(3000),
                    });
                    const m = await res.json();
                    // Map the backend metrics to the shape the dashboard renders.
                    data = {
                        cpu: m.cpu_usage_percent ?? 0,
                        memory: m.memory_usage_mb ?? 0,
                        memories: m.memories ?? 0,
                        facts: m.facts ?? 0,
                        vectors: m.vector_index_size ?? 0,
                        storage_bytes: m.storage_bytes ?? 0,
                        timestamp: Date.now(),
                        source: "backend",
                    };
                } catch {
                    // Backend unreachable — signal it honestly instead of faking.
                    data = { timestamp: Date.now(), source: "unavailable" };
                }
                try {
                    controller.enqueue(encoder.encode(`data: ${JSON.stringify(data)}\n\n`));
                } catch {
                    if (intervalId) clearInterval(intervalId);
                }
            };

            sendMetrics();
            intervalId = setInterval(sendMetrics, 2000);
        },
        cancel() {
            if (intervalId) clearInterval(intervalId);
        },
    });

    return new Response(body, {
        headers: {
            "Content-Type": "text/event-stream",
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
        },
    });
}

// HTML template
function createHtml(): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>GaussOS v3.0 | AI Memory Management Platform</title>
    <link rel="stylesheet" href="/static/styles.css">
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🧠</text></svg>">
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.js"></script>
    <script src="/static/app.js" defer></script>
</head>
<body>
    <div class="app-shell">
        <!-- Sidebar -->
        <aside class="sidebar">
            <div class="logo-section">
                <div class="logo-mark">🧠</div>
                <div class="logo-text">
                    <span class="logo-title">GaussOS</span>
                    <span class="logo-version">v3.0.0</span>
                </div>
            </div>
            
            <nav class="nav-section">
                <div class="nav-group">
                    <div class="nav-group-title">Overview</div>
                    <a class="nav-item active" data-page="dashboard">
                        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="3" y="3" width="7" height="7" rx="1"/>
                            <rect x="14" y="3" width="7" height="7" rx="1"/>
                            <rect x="3" y="14" width="7" height="7" rx="1"/>
                            <rect x="14" y="14" width="7" height="7" rx="1"/>
                        </svg>
                        <span>Dashboard</span>
                    </a>
                </div>

                <div class="nav-group">
                    <div class="nav-group-title">Test the memory</div>
                    <a class="nav-item" data-page="playground">
                        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="11" cy="11" r="7"/>
                            <path d="M21 21l-4.3-4.3"/>
                        </svg>
                        <span>Retrieval Playground</span>
                    </a>
                    <a class="nav-item" data-page="memories">
                        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="10"/>
                            <circle cx="12" cy="12" r="4"/>
                            <path d="M12 2v4"/>
                            <path d="M12 18v4"/>
                            <path d="M2 12h4"/>
                            <path d="M18 12h4"/>
                        </svg>
                        <span>Memories</span>
                    </a>
                    <a class="nav-item" data-page="kg">
                        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="5" cy="6" r="2"/>
                            <circle cx="19" cy="6" r="2"/>
                            <circle cx="12" cy="18" r="2"/>
                            <path d="M7 7l4 9M17 7l-4 9M6 6h12"/>
                        </svg>
                        <span>Knowledge Graph</span>
                    </a>
                </div>

                <div class="nav-group">
                    <div class="nav-group-title">System</div>
                    <a class="nav-item" data-page="settings">
                        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="3"/>
                            <path d="M12 1v4"/>
                            <path d="M12 19v4"/>
                            <path d="M1 12h4"/>
                            <path d="M19 12h4"/>
                            <path d="M4.22 4.22l2.83 2.83"/>
                            <path d="M16.95 16.95l2.83 2.83"/>
                            <path d="M4.22 19.78l2.83-2.83"/>
                            <path d="M16.95 7.05l2.83-2.83"/>
                        </svg>
                        <span>Settings</span>
                    </a>
                </div>
            </nav>
            
            <div class="sidebar-footer">
                <div class="user-card">
                    <div class="user-avatar">A</div>
                    <div class="user-info">
                        <div class="user-name">Admin User</div>
                        <div class="user-role">System Administrator</div>
                    </div>
                </div>
            </div>
        </aside>
        
        <!-- Header -->
        <header class="header">
            <div class="header-left">
                <div class="breadcrumb">
                    <span class="breadcrumb-item">GaussOS</span>
                    <span class="breadcrumb-separator">/</span>
                    <span class="breadcrumb-item current" id="current-page-title">Dashboard</span>
                </div>
            </div>
            
            <div class="search-container">
                <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <path d="M21 21l-4.35-4.35"/>
                </svg>
                <input type="text" class="search-input" placeholder="Search memories, agents, settings...">
                <span class="search-shortcut">⌘K</span>
            </div>
            
            <div class="header-actions">
                <button class="header-btn has-notification" title="Notifications">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
                        <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
                    </svg>
                </button>
                <button class="header-btn" title="Help">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
                        <path d="M12 17h.01"/>
                    </svg>
                </button>
                <button class="header-btn" id="theme-toggle" title="Toggle theme">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="5"/>
                        <path d="M12 1v2"/>
                        <path d="M12 21v2"/>
                        <path d="M4.22 4.22l1.42 1.42"/>
                        <path d="M18.36 18.36l1.42 1.42"/>
                        <path d="M1 12h2"/>
                        <path d="M21 12h2"/>
                        <path d="M4.22 19.78l1.42-1.42"/>
                        <path d="M18.36 5.64l1.42-1.42"/>
                    </svg>
                </button>
            </div>
        </header>
        
        <!-- Main Content -->
        <main class="main-content">
            <!-- Dashboard Page -->
            <div class="page" id="page-dashboard">
                <div class="page-header animate-fade-in">
                    <h1 class="page-title">GaussOS</h1>
                    <p class="page-description">Long-term memory for AI agents — store memories, then see <em>exactly why</em> each one is retrieved. All numbers below are live from your instance.</p>
                </div>

                <!-- Stats Grid (live from /api/v1/metrics) -->
                <div class="stats-grid">
                    <div class="stat-card animate-fade-in stagger-1">
                        <div class="stat-icon">💾</div>
                        <div class="stat-value" id="stat-memories">—</div>
                        <div class="stat-label">Memories stored</div>
                    </div>
                    <div class="stat-card animate-fade-in stagger-2">
                        <div class="stat-icon">🕸️</div>
                        <div class="stat-value" id="stat-facts">—</div>
                        <div class="stat-label">Facts in graph</div>
                    </div>
                    <div class="stat-card animate-fade-in stagger-3">
                        <div class="stat-icon">🧭</div>
                        <div class="stat-value" id="stat-vectors">—</div>
                        <div class="stat-label">Vectors indexed</div>
                    </div>
                    <div class="stat-card animate-fade-in stagger-4">
                        <div class="stat-icon">🤖</div>
                        <div class="stat-value" id="stat-llm" style="font-size:1.1rem;">—</div>
                        <div class="stat-label">LLM provider</div>
                    </div>
                </div>

                <!-- Quick start -->
                <div class="card wide animate-fade-in">
                    <div class="card-header">
                        <h3 class="card-title">
                            <svg class="card-title-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M5 12h14"/><path d="M12 5l7 7-7 7"/>
                            </svg>
                            Get started in 30 seconds
                        </h3>
                    </div>
                    <div class="card-body">
                        <ol style="margin:0 0 1rem; padding-left:1.2rem; line-height:1.9; color:var(--text-secondary,#aaa);">
                            <li><strong>Seed sample data</strong> (or add your own memories) so there's something to search.</li>
                            <li>Open the <strong>Retrieval Playground</strong> and run a query.</li>
                            <li>Compare <strong>lexical (BM25)</strong> vs <strong>vector</strong> vs <strong>hybrid (RRF)</strong> — with full score breakdowns.</li>
                        </ol>
                        <div style="display:flex; gap:.5rem; flex-wrap:wrap;">
                            <button id="dash-seed" class="btn btn-primary">Seed sample data</button>
                            <button id="dash-playground" class="btn btn-secondary">Open Retrieval Playground</button>
                            <button id="dash-kg" class="btn btn-ghost">View Knowledge Graph</button>
                        </div>
                        <p id="dash-seed-out" class="page-description" style="margin-top:.6rem;"></p>
                    </div>
                </div>
            </div>
            
            <!-- Memories Page -->
            <div class="page" id="page-memories" style="display: none;">
                <div class="page-header">
                    <h1 class="page-title">Memory Explorer</h1>
                    <p class="page-description">Browse and filter your AI memory store — faceted by namespace, type, quality, and full-text.</p>
                </div>
                <div class="card">
                    <div class="card-body">
                        <div style="display:flex; gap:0.5rem; align-items:center; flex-wrap:wrap;">
                            <input id="mem-search" class="search-input" style="flex:1; min-width:200px;" placeholder="Search text…" />
                            <input id="mem-namespace" class="search-input" style="max-width:160px;" placeholder="namespace" />
                            <select id="mem-type" class="search-input" style="max-width:150px;">
                                <option value="">any type</option>
                                <option value="text">text</option>
                                <option value="plaintext">plaintext</option>
                                <option value="semantic">semantic</option>
                                <option value="episodic">episodic</option>
                                <option value="procedural">procedural</option>
                            </select>
                            <input id="mem-minq" type="number" min="0" max="1" step="0.1" class="search-input" style="max-width:120px;" placeholder="min quality" />
                            <button id="mem-refresh" class="btn btn-primary">Search</button>
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">Results</h3>
                        <span id="mem-count" class="page-description"></span>
                    </div>
                    <div class="card-body">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>Type</th>
                                    <th>Content</th>
                                    <th>Namespace</th>
                                    <th>Quality</th>
                                    <th>Actions</th>
                                </tr>
                            </thead>
                            <tbody id="memories-list">
                                <!-- Populated by JavaScript -->
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>

            <!-- Retrieval Playground Page -->
            <div class="page" id="page-playground" style="display: none;">
                <div class="page-header">
                    <h1 class="page-title">Retrieval Playground</h1>
                    <p class="page-description">See exactly <em>why</em> each memory is retrieved — compare lexical (BM25), vector, and fused hybrid ranking side by side with full score breakdowns. White-box retrieval no other agent-memory system exposes.</p>
                </div>

                <!-- Step 1: put something in memory -->
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">1 · Add a memory</h3>
                        <button id="pg-seed" class="btn btn-ghost">Seed sample data</button>
                    </div>
                    <div class="card-body">
                        <div style="display:flex; gap:0.5rem; align-items:center; flex-wrap:wrap;">
                            <input id="pg-add-text" class="search-input" style="flex:1; min-width:260px;" placeholder="e.g. Alice prefers dark roast coffee" />
                            <input id="pg-add-ns" class="search-input" style="max-width:180px;" placeholder="namespace (default: demo)" />
                            <button id="pg-add" class="btn btn-secondary">Add memory</button>
                        </div>
                        <p id="pg-add-out" class="page-description" style="margin-top:0.5rem;"></p>
                    </div>
                </div>

                <!-- Step 2: query it -->
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">2 · Run a query</h3>
                    </div>
                    <div class="card-body">
                        <div style="display:flex; gap:0.75rem; align-items:center; flex-wrap:wrap;">
                            <input id="pg-query" class="search-input" style="flex:1; min-width:240px;" placeholder="e.g. what coffee does alice like?" />
                            <input id="pg-namespace" class="search-input" style="max-width:180px;" placeholder="namespace (optional)" />
                            <button id="pg-run" class="btn btn-primary">Run comparison</button>
                        </div>
                        <div id="pg-examples" style="margin-top:0.6rem; display:flex; gap:0.4rem; flex-wrap:wrap; align-items:center;">
                            <span class="page-description" style="margin:0;">Try:</span>
                        </div>
                        <p id="pg-meta" class="page-description" style="margin-top:0.5rem;"></p>
                    </div>
                </div>

                <div id="pg-results" style="display:grid; grid-template-columns:repeat(auto-fit,minmax(280px,1fr)); gap:1rem;"></div>
            </div>

            <!-- Knowledge Graph Page -->
            <div class="page" id="page-kg" style="display: none;">
                <div class="page-header">
                    <h1 class="page-title">Knowledge Graph</h1>
                    <p class="page-description">The bi-temporal entity graph. Drag the time slider to see the graph <em>as it was valid</em> at any point — temporal recall no other agent-memory UI shows. Click a node to trace multi-hop relevance (Personalized PageRank).</p>
                </div>
                <div class="card">
                    <div class="card-body">
                        <div style="display:flex; gap:0.75rem; align-items:center; flex-wrap:wrap;">
                            <label class="page-description" style="margin:0;">As of:</label>
                            <input id="kg-asof" type="range" min="0" max="100" value="100" style="flex:1; min-width:200px;" />
                            <span id="kg-asof-label" class="page-description" style="margin:0; font-family:var(--font-mono,monospace);">now</span>
                            <button id="kg-refresh" class="btn btn-primary">Refresh</button>
                        </div>
                        <p id="kg-meta" class="page-description" style="margin-top:0.5rem;"></p>
                    </div>
                </div>
                <div class="card">
                    <div class="card-body">
                        <canvas id="kg-canvas" width="900" height="520" style="width:100%; height:auto; background:var(--surface-2,#1a1d2e); border-radius:8px;"></canvas>
                    </div>
                </div>
            </div>

            <!-- Settings Page -->
            <div class="page" id="page-settings" style="display: none;">
                <div class="page-header">
                    <h1 class="page-title">Settings</h1>
                    <p class="page-description">Configure your GaussOS instance</p>
                </div>
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">LLM Provider</h3>
                    </div>
                    <div class="card-body">
                        <p id="settings-llm" class="text-secondary">Loading…</p>
                    </div>
                </div>
                <div class="card">
                    <div class="card-header">
                        <h3 class="card-title">Memory Maintenance</h3>
                    </div>
                    <div class="card-body">
                        <p class="page-description">Run a sleep-time forgetting pass: cold memories are archived and (optionally) the lowest-retention ones are forgotten — the cognitive housekeeping that keeps memory bounded.</p>
                        <div style="display:flex; gap:0.5rem; align-items:center; flex-wrap:wrap;">
                            <input id="forget-ns" class="search-input" style="max-width:200px;" placeholder="namespace (e.g. demo)" />
                            <label class="page-description" style="margin:0;"><input id="forget-delete" type="checkbox" /> delete forgotten</label>
                            <button id="forget-run" class="btn btn-primary">Run forgetting pass</button>
                        </div>
                        <p id="forget-out" class="page-description" style="margin-top:0.5rem; font-family:var(--font-mono,monospace);"></p>
                    </div>
                </div>
            </div>
        </main>
    </div>
    
</body>
</html>`;
}

// Start server
console.log("🧠 GaussOS Web UI v3.0");
console.log(`   Listening on http://localhost:${PORT}`);
console.log("   Press Ctrl+C to stop\n");

Deno.serve({ port: PORT }, handler);
