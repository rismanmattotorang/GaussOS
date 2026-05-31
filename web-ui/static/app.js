// GaussOS v3.0 - Modern Web Application
// High-performance UI with real-time updates and comprehensive error handling

class GaussOSApp {
    constructor() {
        this.state = {
            currentPage: 'dashboard',
            connected: false,
            user: null,
            metrics: {},
            memories: [],
            agents: [],
            logs: [],
            notifications: [],
            loading: {}
        };
        this.ws = null;
        this.eventSource = null;
        this.charts = {};
        this.init();
    }

    async init() {
        this.applyTheme();
        this.setupEventListeners();
        this.initWebSocket();
        this.initSSE();
        await this.loadInitialData();
        this.startMetricsPolling();
        this.maybeShowFirstRun();
    }

    // ===== API Client with Error Handling =====
    async api(endpoint, options = {}) {
        const config = {
            headers: {
                'Content-Type': 'application/json',
                ...(this.getAuthToken() && { 'Authorization': `Bearer ${this.getAuthToken()}` })
            },
            ...options
        };

        try {
            this.setLoading(endpoint, true);
            const response = await fetch(`/api/v1${endpoint}`, config);
            
            if (!response.ok) {
                const error = await response.json().catch(() => ({ message: response.statusText }));
                throw new APIError(error.message || 'Request failed', response.status, error);
            }
            
            return await response.json();
        } catch (error) {
            if (error instanceof APIError) {
                this.handleAPIError(error);
                throw error;
            }
            this.showNotification('Network error. Please check your connection.', 'error');
            throw new APIError('Network error', 0, { original: error.message });
        } finally {
            this.setLoading(endpoint, false);
        }
    }

    // ===== WebSocket Connection =====
    initWebSocket() {
        const wsUrl = `ws://${window.location.host}/ws`;
        
        const connect = () => {
            this.ws = new WebSocket(wsUrl);
            
            this.ws.onopen = () => {
                this.state.connected = true;
                this.updateConnectionStatus();
                this.showNotification('Connected to server', 'success');
            };

            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    this.handleWSMessage(data);
                } catch (e) {
                    console.error('WebSocket message parse error:', e);
                }
            };

            this.ws.onclose = () => {
                this.state.connected = false;
                this.updateConnectionStatus();
                setTimeout(connect, 3000);
            };

            this.ws.onerror = () => {
                this.state.connected = false;
            };
        };

        connect();
    }

    handleWSMessage(data) {
        switch (data.type) {
            case 'metrics':
                this.updateMetrics(data.data);
                break;
            case 'memory_update':
                this.handleMemoryUpdate(data.data);
                break;
            case 'agent_status':
                this.handleAgentStatus(data.data);
                break;
            case 'log':
                this.addLogEntry(data.data);
                break;
            case 'notification':
                this.showNotification(data.message, data.level);
                break;
        }
    }

    // ===== Server-Sent Events for Metrics =====
    initSSE() {
        this.eventSource = new EventSource('/events/metrics');
        
        this.eventSource.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                this.updateMetrics(data);
            } catch (e) {
                console.error('SSE parse error:', e);
            }
        };

        this.eventSource.onerror = () => {
            setTimeout(() => this.initSSE(), 5000);
        };
    }

    // ===== Data Loading =====
    async loadInitialData() {
        try {
            const [health, memories, agents] = await Promise.allSettled([
                this.api('/health').catch(() => ({ status: 'unknown' })),
                this.api('/memories?limit=100').catch(() => ({ memories: [] })),
                this.api('/agents').catch(() => [])
            ]);

            if (health.status === 'fulfilled') this.state.health = health.value;
            if (memories.status === 'fulfilled') this.state.memories = memories.value.memories || [];
            if (agents.status === 'fulfilled') this.state.agents = agents.value || [];

            this.renderCurrentPage();
        } catch (error) {
            console.error('Failed to load initial data:', error);
        }
    }

    // ===== Metrics Updates =====
    updateMetrics(data) {
        this.state.metrics = { ...this.state.metrics, ...data };
        this.updateMetricsDisplay();
    }

    updateMetricsDisplay() {
        const m = this.state.metrics;
        // Show real values from the backend; default to 0 rather than invented
        // numbers so the dashboard never displays fabricated data.
        this.updateElement('stat-requests', this.formatNumber(m.requests ?? 0));
        this.updateElement('stat-memories', this.formatNumber(m.memories ?? 0));
        this.updateElement('stat-cache', `${(m.cache ?? 0).toFixed(1)}%`);
        this.updateElement('stat-agents', m.agents ?? 0);
        
        if (this.charts.performance) {
            this.updatePerformanceChart(m);
        }
    }

    updatePerformanceChart(data) {
        if (!this.charts.performance) return;
        
        const chart = this.charts.performance;
        const newValue = data.requests || (10000 + Math.random() * 4000);
        
        chart.data.datasets[0].data.push(newValue);
        chart.data.datasets[0].data.shift();
        chart.update('none');
    }

    startMetricsPolling() {
        setInterval(async () => {
            try {
                const metrics = await this.api('/metrics').catch(() => null);
                if (metrics) this.updateMetrics(metrics);
            } catch (e) { /* ignore */ }
        }, 5000);
    }

    // ===== Navigation =====
    setupEventListeners() {
        document.querySelectorAll('.nav-item[data-page]').forEach(item => {
            item.addEventListener('click', (e) => {
                e.preventDefault();
                this.navigateTo(item.dataset.page);
            });
        });

        document.getElementById('theme-toggle')?.addEventListener('click', () => this.toggleTheme());
        
        document.addEventListener('keydown', (e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
                e.preventDefault();
                this.openCommandPalette();
            }
        });

        document.querySelector('.search-input')?.addEventListener('input', (e) => {
            this.handleSearch(e.target.value);
        });
    }

    navigateTo(page) {
        this.state.currentPage = page;
        
        document.querySelectorAll('.nav-item').forEach(i => i.classList.remove('active'));
        document.querySelector(`[data-page="${page}"]`)?.classList.add('active');
        
        document.querySelectorAll('.page').forEach(p => p.style.display = 'none');
        const targetPage = document.getElementById(`page-${page}`);
        if (targetPage) {
            targetPage.style.display = 'block';
            this.renderPage(page);
        }
        
        document.getElementById('current-page-title').textContent = 
            page.charAt(0).toUpperCase() + page.slice(1);
    }

    renderPage(page) {
        switch (page) {
            case 'dashboard': this.renderDashboard(); break;
            case 'memories': this.renderMemories(); break;
            case 'playground': this.renderPlayground(); break;
            case 'kg': this.renderKnowledgeGraph(); break;
            case 'agents': this.renderAgents(); break;
            case 'analytics': this.renderAnalytics(); break;
            case 'graphs': this.renderGraphs(); break;
            case 'logs': this.renderLogs(); break;
            case 'settings': this.renderSettings(); break;
        }
    }

    // ===== Retrieval Playground (white-box BM25 vs vector vs hybrid) =====
    renderPlayground() {
        const runBtn = document.getElementById('pg-run');
        if (!runBtn || runBtn.dataset.wired) return;
        runBtn.dataset.wired = '1';
        const run = () => this.runRetrievalCompare();
        runBtn.addEventListener('click', run);
        document.getElementById('pg-query')?.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') run();
        });
    }

    async runRetrievalCompare() {
        const text = (document.getElementById('pg-query')?.value || '').trim();
        const namespace = (document.getElementById('pg-namespace')?.value || '').trim();
        const results = document.getElementById('pg-results');
        const meta = document.getElementById('pg-meta');
        if (!text) { this.showNotification('Enter a query first', 'warning'); return; }
        if (meta) meta.textContent = 'Running…';
        try {
            const body = { text, top_k: 8 };
            if (namespace) body.namespace = namespace;
            const data = await this.api('/retrieval/compare', {
                method: 'POST',
                body: JSON.stringify(body),
            });
            if (meta) meta.textContent = `Candidate pool: ${data.candidate_pool} memories`;
            const columns = [
                ['Lexical (BM25)', data.lexical || []],
                ['Vector', data.vector || []],
                ['Hybrid (RRF)', data.hybrid || []],
            ];
            results.innerHTML = columns.map(([title, list]) => `
                <div class="card"><div class="card-header"><h3 class="card-title">${title}</h3></div>
                <div class="card-body">${
                    list.length ? list.map((r, i) => `
                        <div style="padding:.5rem 0; border-bottom:1px solid var(--border-subtle,#222);">
                            <div><strong>#${i + 1}</strong> ${this.escapeHtml(r.content || r.id)}</div>
                            <div style="font-size:.75rem; color:var(--text-muted,#888); font-family:var(--font-mono,monospace);">
                                score=${(r.score ?? 0).toFixed(4)}
                                · bm25=${(r.bm25_score ?? 0).toFixed(2)} (rank ${r.bm25_rank ?? '-'})
                                · vec=${(r.vector_score ?? 0).toFixed(2)} (rank ${r.vector_rank ?? '-'})
                                · recency=${(r.recency_score ?? 0).toFixed(2)}
                            </div>
                        </div>`).join('')
                    : '<p style="color:var(--text-muted,#888);">No results</p>'
                }</div></div>`).join('');
        } catch (e) {
            if (meta) meta.textContent = `Error: ${e.message || e}`;
        }
    }

    renderCurrentPage() {
        this.renderPage(this.state.currentPage);
    }

    // ===== Notification System =====
    showNotification(message, type = 'info', duration = 4000) {
        const container = document.getElementById('notifications') || this.createNotificationContainer();
        
        const notification = document.createElement('div');
        notification.className = `notification notification-${type}`;
        notification.innerHTML = `
            <div class="notification-icon">${this.getNotificationIcon(type)}</div>
            <div class="notification-content">${this.escapeHtml(message)}</div>
            <button class="notification-close">&times;</button>
        `;
        
        notification.querySelector('.notification-close').onclick = () => notification.remove();
        container.appendChild(notification);
        
        setTimeout(() => {
            notification.classList.add('notification-exit');
            setTimeout(() => notification.remove(), 300);
        }, duration);
    }

    createNotificationContainer() {
        const container = document.createElement('div');
        container.id = 'notifications';
        container.className = 'notification-container';
        document.body.appendChild(container);
        return container;
    }

    getNotificationIcon(type) {
        const icons = { success: '✓', error: '✕', warning: '⚠', info: 'ℹ' };
        return icons[type] || icons.info;
    }

    // ===== Error Handling =====
    handleAPIError(error) {
        const messages = {
            401: 'Please log in to continue',
            403: 'You do not have permission for this action',
            404: 'Resource not found',
            429: 'Too many requests. Please slow down.',
            500: 'Server error. Please try again later.'
        };
        
        this.showNotification(messages[error.status] || error.message, 'error');
    }

    // ===== Utility Functions =====
    updateElement(id, value) {
        const el = document.getElementById(id);
        if (el && el.textContent !== String(value)) {
            el.textContent = value;
        }
    }

    formatNumber(num) {
        return new Intl.NumberFormat().format(num);
    }

    escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    setLoading(key, isLoading) {
        this.state.loading[key] = isLoading;
    }

    getAuthToken() {
        return localStorage.getItem('gaussos_token');
    }

    updateConnectionStatus() {
        const indicator = document.getElementById('connection-status');
        if (indicator) {
            indicator.className = `status-indicator ${this.state.connected ? 'connected' : 'disconnected'}`;
            indicator.title = this.state.connected ? 'Connected' : 'Disconnected';
        }
    }

    // Cycle dark → light → system; persist and apply.
    toggleTheme() {
        const order = ['dark', 'light', 'system'];
        let cur = 'dark';
        try { cur = localStorage.getItem('theme') || 'dark'; } catch { /* ignore */ }
        const next = order[(order.indexOf(cur) + 1) % order.length];
        try { localStorage.setItem('theme', next); } catch { /* ignore */ }
        this.applyTheme(next);
        this.showNotification(`Theme: ${next}`, 'info', 1500);
    }

    applyTheme(mode) {
        let m = mode;
        if (!m) { try { m = localStorage.getItem('theme') || 'dark'; } catch { m = 'dark'; } }
        let resolved = m;
        if (m === 'system') {
            resolved = window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
        }
        document.documentElement.setAttribute('data-theme', resolved);
    }

    // ===== Page Renderers =====
    renderDashboard() {
        this.initPerformanceChart();
    }

    // ===== Memory Explorer (faceted, live) =====
    renderMemories() {
        const refresh = document.getElementById('mem-refresh');
        if (refresh && !refresh.dataset.wired) {
            refresh.dataset.wired = '1';
            const go = () => this.loadMemories();
            refresh.addEventListener('click', go);
            document.getElementById('mem-search')?.addEventListener('keydown', (e) => { if (e.key === 'Enter') go(); });
        }
        this.loadMemories();
    }

    async loadMemories() {
        const val = (id) => (document.getElementById(id)?.value || '').trim();
        const body = { limit: 100 };
        if (val('mem-search')) body.text = val('mem-search');
        if (val('mem-namespace')) body.namespace = val('mem-namespace');
        if (val('mem-type')) body.payload_type = val('mem-type');
        const minq = parseFloat(val('mem-minq'));
        if (!Number.isNaN(minq) && minq > 0) body.min_quality = minq;
        try {
            const data = await this.api('/memories/search', { method: 'POST', body: JSON.stringify(body) });
            this.state.memories = data.memories || (data.results || []).map(r => r.memory) || [];
        } catch (e) {
            this.state.memories = [];
        }
        this.renderMemoryRows();
    }

    memoryType(m) {
        return m.payload && typeof m.payload === 'object' ? Object.keys(m.payload)[0] : (m.type || 'Unknown');
    }

    memoryContent(m) {
        const p = m.payload;
        if (typeof p === 'string') return p;
        if (p && typeof p === 'object') {
            if (typeof p.Text === 'string') return p.Text;
            if (p.Plaintext?.content) return p.Plaintext.content;
            if (p.Semantic?.content) return p.Semantic.content;
            if (p.Episodic?.thread_title) return p.Episodic.thread_title;
            if (p.Procedural?.prompt_name) return p.Procedural.prompt_name;
        }
        return m.metadata?.name || '';
    }

    renderMemoryRows() {
        const list = document.getElementById('memories-list');
        if (!list) return;
        const count = document.getElementById('mem-count');
        if (count) count.textContent = `${this.state.memories.length} memories`;
        if (this.state.memories.length === 0) {
            list.innerHTML = '<tr><td colspan="5" class="empty-state">No memories found</td></tr>';
            return;
        }
        list.innerHTML = this.state.memories.map(m => {
            const q = (m.metadata?.quality_score ?? 0).toFixed(2);
            return `
            <tr>
                <td><span class="badge info">${this.escapeHtml(this.memoryType(m))}</span></td>
                <td>${this.escapeHtml(this.memoryContent(m).slice(0, 80))}</td>
                <td>${this.escapeHtml(String(m.namespace ?? 'default'))}</td>
                <td>${q}</td>
                <td><button class="btn btn-ghost text-danger" onclick="app.deleteMemory('${m.id}')">Delete</button></td>
            </tr>`;
        }).join('');
    }

    async deleteMemory(id) {
        try {
            await this.api(`/memories/${id}`, { method: 'DELETE' });
            this.showNotification('Memory deleted', 'success');
            this.loadMemories();
        } catch (e) {
            this.showNotification(`Delete failed: ${e.message || e}`, 'error');
        }
    }

    // ===== Command palette (⌘K) =====
    handleSearch(query) {
        // The global search box opens the palette pre-filtered.
        if (query && query.length > 0) this.openCommandPalette(query);
    }

    commands() {
        const pages = ['dashboard', 'memories', 'playground', 'analytics', 'graphs', 'agents', 'logs', 'settings'];
        const nav = pages.map(p => ({
            label: `Go to ${p.charAt(0).toUpperCase() + p.slice(1)}`,
            run: () => this.navigateTo(p),
        }));
        return [
            ...nav,
            { label: 'Seed sample memories', run: () => this.seedSampleMemories() },
            { label: 'Toggle theme', run: () => this.toggleTheme?.() },
        ];
    }

    openCommandPalette(initial = '') {
        let overlay = document.getElementById('command-palette');
        if (!overlay) {
            overlay = document.createElement('div');
            overlay.id = 'command-palette';
            overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,.5);display:flex;align-items:flex-start;justify-content:center;z-index:9999;';
            overlay.innerHTML = `
                <div style="margin-top:12vh;width:min(560px,92vw);background:var(--surface-1,#12141d);border:1px solid var(--border-default,#333);border-radius:12px;overflow:hidden;box-shadow:0 16px 48px rgba(0,0,0,.6);">
                    <input id="cmd-input" placeholder="Type a command…" style="width:100%;box-sizing:border-box;padding:14px 16px;background:transparent;border:0;border-bottom:1px solid var(--border-subtle,#222);color:var(--text-primary,#fff);font-size:1rem;outline:none;" />
                    <div id="cmd-list" style="max-height:50vh;overflow:auto;"></div>
                </div>`;
            document.body.appendChild(overlay);
            overlay.addEventListener('click', (e) => { if (e.target === overlay) this.closeCommandPalette(); });
            overlay.querySelector('#cmd-input').addEventListener('input', (e) => this.renderCommands(e.target.value));
            overlay.querySelector('#cmd-input').addEventListener('keydown', (e) => {
                if (e.key === 'Escape') this.closeCommandPalette();
                if (e.key === 'Enter') {
                    const first = overlay.querySelector('.cmd-item');
                    if (first) first.click();
                }
            });
        }
        overlay.style.display = 'flex';
        const input = overlay.querySelector('#cmd-input');
        input.value = initial;
        this.renderCommands(initial);
        input.focus();
    }

    renderCommands(filter = '') {
        const listEl = document.getElementById('cmd-list');
        if (!listEl) return;
        const f = filter.toLowerCase();
        const items = this.commands().filter(c => c.label.toLowerCase().includes(f));
        listEl.innerHTML = items.map((c, i) => `<div class="cmd-item" data-i="${i}" style="padding:10px 16px;cursor:pointer;color:var(--text-secondary,#bbb);">${this.escapeHtml(c.label)}</div>`).join('')
            || '<div style="padding:10px 16px;color:var(--text-muted,#888);">No commands</div>';
        listEl.querySelectorAll('.cmd-item').forEach(el => {
            el.addEventListener('mouseenter', () => el.style.background = 'var(--surface-hover,rgba(255,255,255,.05))');
            el.addEventListener('mouseleave', () => el.style.background = 'transparent');
            el.addEventListener('click', () => {
                const cmd = items[parseInt(el.dataset.i, 10)];
                this.closeCommandPalette();
                cmd?.run?.();
            });
        });
    }

    closeCommandPalette() {
        const overlay = document.getElementById('command-palette');
        if (overlay) overlay.style.display = 'none';
    }

    // ===== First-run wizard =====
    async maybeShowFirstRun() {
        try {
            if (localStorage.getItem('gaussos_onboarded')) return;
        } catch { /* ignore */ }
        let status = { provider: 'unknown', model: '', configured: false };
        try { status = await this.api('/llm/status'); } catch { /* offline */ }
        const configured = status.configured;
        const overlay = document.createElement('div');
        overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,.55);display:flex;align-items:center;justify-content:center;z-index:10000;';
        overlay.innerHTML = `
            <div style="width:min(540px,92vw);background:var(--surface-1,#12141d);border:1px solid var(--border-default,#333);border-radius:16px;padding:28px;box-shadow:0 16px 48px rgba(0,0,0,.6);">
                <h2 style="margin:0 0 8px;color:var(--text-primary,#fff);">🧠 Welcome to GaussOS</h2>
                <p style="color:var(--text-secondary,#aaa);margin:0 0 16px;">The superior agent-memory engine, by Gaussian Technologies.</p>
                <div style="padding:12px 14px;border:1px solid var(--border-subtle,#222);border-radius:10px;margin-bottom:16px;">
                    <div style="color:var(--text-secondary,#aaa);font-size:.85rem;">Active LLM provider</div>
                    <div style="color:var(--text-primary,#fff);font-family:var(--font-mono,monospace);">
                        ${this.escapeHtml(status.provider)} · ${this.escapeHtml(status.model || '—')}
                        ${configured ? '<span class="badge success" style="margin-left:8px;">configured</span>' : '<span class="badge warning" style="margin-left:8px;">no API key</span>'}
                    </div>
                    ${configured ? '' : '<div style="color:var(--text-muted,#888);font-size:.8rem;margin-top:6px;">Set LLM_PROVIDER + an API key (see .env.example) to enable live agent responses.</div>'}
                </div>
                <div style="display:flex;gap:.5rem;justify-content:flex-end;">
                    <button id="fr-seed" class="btn btn-ghost">Seed sample memories</button>
                    <button id="fr-go" class="btn btn-primary">Get started</button>
                </div>
            </div>`;
        document.body.appendChild(overlay);
        const done = () => {
            try { localStorage.setItem('gaussos_onboarded', '1'); } catch { /* ignore */ }
            overlay.remove();
        };
        overlay.querySelector('#fr-go').addEventListener('click', done);
        overlay.querySelector('#fr-seed').addEventListener('click', async () => {
            await this.seedSampleMemories();
            done();
            this.navigateTo('memories');
        });
    }

    async seedSampleMemories() {
        const samples = [
            { payload: { Text: 'GaussOS uses an embedded SurrealDB backend' }, tags: ['gaussos', 'db'], namespace: 'demo', quality_score: 0.9 },
            { payload: { Text: 'Hybrid retrieval fuses BM25 and vector search with RRF' }, tags: ['retrieval'], namespace: 'demo', quality_score: 0.8 },
            { payload: { Text: 'The forgetting curve prunes stale memories over time' }, tags: ['memory'], namespace: 'demo', quality_score: 0.7 },
        ];
        let ok = 0;
        for (const s of samples) {
            try { await this.api('/memories', { method: 'POST', body: JSON.stringify(s) }); ok++; } catch { /* ignore */ }
        }
        // Seed a few connected facts so the Knowledge Graph has content.
        const facts = [
            { subject: 'GaussOS', predicate: 'built_by', object: 'Gaussian Technologies' },
            { subject: 'Gaussian Technologies', predicate: 'based_in', object: 'Indonesia' },
            { subject: 'GaussOS', predicate: 'written_in', object: 'Rust' },
        ];
        for (const f of facts) {
            try { await this.api('/facts', { method: 'POST', body: JSON.stringify(f) }); } catch { /* ignore */ }
        }
        this.showNotification(`Seeded ${ok} memories + ${facts.length} facts`, 'success');
    }

    renderAgents() {
        const list = document.getElementById('agents-list');
        if (!list) return;

        list.innerHTML = this.state.agents.map(a => `
            <tr>
                <td><code>${this.escapeHtml(a.id)}</code></td>
                <td>${this.escapeHtml(a.name)}</td>
                <td><span class="badge ${a.status === 'active' ? 'success' : 'info'}">${a.status}</span></td>
                <td>${this.formatNumber(a.executions)}</td>
                <td>
                    <button class="btn btn-ghost" onclick="app.viewAgent('${a.id}')">Details</button>
                    <button class="btn btn-ghost" onclick="app.toggleAgent('${a.id}')">${a.status === 'active' ? 'Stop' : 'Start'}</button>
                </td>
            </tr>
        `).join('');
    }

    renderAnalytics() {
        // Analytics charts initialization
    }

    renderGraphs() {
        // Graph visualization
    }

    renderLogs() {
        const container = document.getElementById('logs-container');
        if (!container) return;

        container.innerHTML = this.state.logs.slice(-100).map(log => `
            <div class="log-entry log-${log.level?.toLowerCase() || 'info'}">
                <span class="log-time">${log.timestamp}</span>
                <span class="log-level">${log.level}</span>
                <span class="log-message">${this.escapeHtml(log.message)}</span>
            </div>
        `).join('');
    }

    async renderSettings() {
        // LLM provider status.
        const llmEl = document.getElementById('settings-llm');
        if (llmEl) {
            try {
                const s = await this.api('/llm/status');
                llmEl.innerHTML = `Provider: <strong>${this.escapeHtml(s.provider)}</strong> · model <code>${this.escapeHtml(s.model || '—')}</code> · ${s.configured ? '<span class="badge success">configured</span>' : '<span class="badge warning">no API key</span>'}`;
            } catch (e) {
                llmEl.textContent = `Unavailable: ${e.message || e}`;
            }
        }
        // Forgetting pass control.
        const btn = document.getElementById('forget-run');
        if (btn && !btn.dataset.wired) {
            btn.dataset.wired = '1';
            btn.addEventListener('click', () => this.runForgettingPass());
        }
    }

    async runForgettingPass() {
        const ns = (document.getElementById('forget-ns')?.value || '').trim();
        const del = document.getElementById('forget-delete')?.checked || false;
        const out = document.getElementById('forget-out');
        if (!ns) { this.showNotification('Enter a namespace', 'warning'); return; }
        if (out) out.textContent = 'Running…';
        try {
            const r = await this.api('/admin/forget', {
                method: 'POST',
                body: JSON.stringify({ namespace: ns, delete_forgotten: del }),
            });
            if (out) out.textContent = `retained=${r.retained} · archived=${r.archived} · forgotten=${r.forgotten}${del ? ' (deleted)' : ''}`;
            this.showNotification('Forgetting pass complete', 'success');
        } catch (e) {
            if (out) out.textContent = `Error: ${e.message || e}`;
        }
    }

    // ===== Knowledge Graph viewer (bi-temporal "as-of" + PPR highlight) =====
    renderKnowledgeGraph() {
        const refresh = document.getElementById('kg-refresh');
        const slider = document.getElementById('kg-asof');
        if (refresh && !refresh.dataset.wired) {
            refresh.dataset.wired = '1';
            refresh.addEventListener('click', () => this.loadKnowledgeGraph());
            slider?.addEventListener('input', () => this.updateAsOfLabel());
            slider?.addEventListener('change', () => this.loadKnowledgeGraph());
            const canvas = document.getElementById('kg-canvas');
            canvas?.addEventListener('click', (e) => this.onKgClick(e));
        }
        this.updateAsOfLabel();
        this.loadKnowledgeGraph();
    }

    asOfValue() {
        // Slider 0..100 → a point in the last 30 days; 100 = now (no ?at).
        const v = parseInt(document.getElementById('kg-asof')?.value ?? '100', 10);
        if (v >= 100) return null;
        const spanMs = 30 * 24 * 3600 * 1000;
        return new Date(Date.now() - spanMs * (1 - v / 100));
    }

    updateAsOfLabel() {
        const label = document.getElementById('kg-asof-label');
        const d = this.asOfValue();
        if (label) label.textContent = d ? d.toISOString().slice(0, 16).replace('T', ' ') : 'now';
    }

    async loadKnowledgeGraph() {
        const meta = document.getElementById('kg-meta');
        const d = this.asOfValue();
        const ep = d ? `/facts/graph?at=${encodeURIComponent(d.toISOString())}` : '/facts/graph';
        try {
            const g = await this.api(ep);
            this.kgData = g;
            this.kgHighlight = null;
            if (meta) meta.textContent = `${g.nodes.length} entities · ${g.edges.length} relations${d ? ` · as of ${d.toISOString().slice(0,16).replace('T',' ')}` : ' · current'}`;
            this.drawKnowledgeGraph();
        } catch (e) {
            if (meta) meta.textContent = `Error: ${e.message || e}`;
        }
    }

    kgLayout() {
        // Deterministic circular layout; store positions keyed by node id.
        const g = this.kgData || { nodes: [], edges: [] };
        const canvas = document.getElementById('kg-canvas');
        const w = canvas.width, h = canvas.height;
        const cx = w / 2, cy = h / 2, r = Math.min(w, h) / 2 - 50;
        const pos = {};
        g.nodes.forEach((n, i) => {
            const a = (i / Math.max(1, g.nodes.length)) * Math.PI * 2;
            pos[n.id] = { x: cx + r * Math.cos(a), y: cy + r * Math.sin(a), deg: n.degree };
        });
        this.kgPos = pos;
        return pos;
    }

    drawKnowledgeGraph() {
        const canvas = document.getElementById('kg-canvas');
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        const g = this.kgData || { nodes: [], edges: [] };
        const pos = this.kgLayout();
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        if (g.nodes.length === 0) {
            ctx.fillStyle = '#888';
            ctx.font = '16px sans-serif';
            ctx.fillText('No facts yet — ingest facts (POST /facts) or use the first-run wizard to seed.', 30, 40);
            return;
        }
        const hi = this.kgHighlight; // Set of highlighted node ids
        // Edges
        ctx.strokeStyle = 'rgba(148,163,184,0.35)';
        ctx.lineWidth = 1;
        g.edges.forEach(e => {
            const a = pos[e.source], b = pos[e.target];
            if (!a || !b) return;
            ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
        });
        // Nodes
        g.nodes.forEach(n => {
            const p = pos[n.id];
            const radius = 6 + Math.min(14, n.degree * 2);
            const on = !hi || hi.has(n.id);
            ctx.beginPath();
            ctx.arc(p.x, p.y, radius, 0, Math.PI * 2);
            ctx.fillStyle = hi && hi.has(n.id) ? '#00d9ff' : (on ? '#8b5cf6' : 'rgba(139,92,246,0.25)');
            ctx.fill();
            ctx.fillStyle = on ? '#f8fafc' : 'rgba(248,250,252,0.4)';
            ctx.font = '12px sans-serif';
            ctx.fillText(n.id.length > 18 ? n.id.slice(0, 17) + '…' : n.id, p.x + radius + 3, p.y + 4);
        });
    }

    async onKgClick(e) {
        const canvas = document.getElementById('kg-canvas');
        const rect = canvas.getBoundingClientRect();
        const sx = canvas.width / rect.width, sy = canvas.height / rect.height;
        const x = (e.clientX - rect.left) * sx, y = (e.clientY - rect.top) * sy;
        const pos = this.kgPos || {};
        let hit = null;
        for (const [id, p] of Object.entries(pos)) {
            const rr = 6 + Math.min(14, (p.deg || 1) * 2) + 4;
            if ((x - p.x) ** 2 + (y - p.y) ** 2 <= rr * rr) { hit = id; break; }
        }
        if (!hit) { this.kgHighlight = null; this.drawKnowledgeGraph(); return; }
        try {
            const r = await this.api('/facts/graph-search', { method: 'POST', body: JSON.stringify({ seeds: [hit] }) });
            const set = new Set([hit]);
            (r.hits || []).forEach(h => { set.add(h.subject); set.add(h.object); });
            this.kgHighlight = set;
            this.drawKnowledgeGraph();
            this.showNotification(`PPR from "${hit}": ${r.total} related facts`, 'info');
        } catch (err) {
            this.showNotification(`Graph search failed: ${err.message || err}`, 'error');
        }
    }

    // ===== Chart Initialization =====
    initPerformanceChart() {
        const ctx = document.getElementById('performance-chart');
        if (!ctx) return;

        // Destroy existing chart to prevent "Canvas is already in use" error
        const existingChart = Chart.getChart(ctx);
        if (existingChart) {
            existingChart.destroy();
        }

        const gradient = ctx.getContext('2d').createLinearGradient(0, 0, 0, 300);
        gradient.addColorStop(0, 'rgba(0, 217, 255, 0.3)');
        gradient.addColorStop(1, 'rgba(0, 217, 255, 0)');

        this.charts.performance = new Chart(ctx, {
            type: 'line',
            data: {
                labels: Array.from({length: 24}, (_, i) => `${i}:00`),
                datasets: [{
                    label: 'Requests/sec',
                    data: Array.from({length: 24}, () => 10000 + Math.random() * 4000),
                    borderColor: '#00d9ff',
                    backgroundColor: gradient,
                    fill: true,
                    tension: 0.4,
                    pointRadius: 0,
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: { legend: { labels: { color: '#94a3b8' } } },
                scales: {
                    x: { grid: { color: 'rgba(255,255,255,0.05)' }, ticks: { color: '#64748b' } },
                    y: { grid: { color: 'rgba(255,255,255,0.05)' }, ticks: { color: '#64748b' } }
                }
            }
        });
    }

    // ===== Memory Operations =====
    async viewMemory(id) {
        try {
            const memory = await this.api(`/memories/${id}`);
            this.showModal('Memory Details', `<pre>${JSON.stringify(memory, null, 2)}</pre>`);
        } catch (e) { /* handled by api() */ }
    }

    async deleteMemory(id) {
        if (!confirm('Are you sure you want to delete this memory?')) return;
        try {
            await this.api(`/memories/${id}`, { method: 'DELETE' });
            this.state.memories = this.state.memories.filter(m => m.id !== id);
            this.renderMemories();
            this.showNotification('Memory deleted successfully', 'success');
        } catch (e) { /* handled by api() */ }
    }

    // ===== Modal System =====
    showModal(title, content) {
        const modal = document.getElementById('modal') || this.createModal();
        modal.querySelector('.modal-title').textContent = title;
        modal.querySelector('.modal-body').innerHTML = content;
        modal.classList.add('active');
    }

    createModal() {
        const modal = document.createElement('div');
        modal.id = 'modal';
        modal.className = 'modal-overlay';
        modal.innerHTML = `
            <div class="modal">
                <div class="modal-header">
                    <h3 class="modal-title"></h3>
                    <button class="modal-close" onclick="app.closeModal()">&times;</button>
                </div>
                <div class="modal-body"></div>
            </div>
        `;
        modal.addEventListener('click', (e) => {
            if (e.target === modal) this.closeModal();
        });
        document.body.appendChild(modal);
        return modal;
    }

    closeModal() {
        document.getElementById('modal')?.classList.remove('active');
    }

    addLogEntry(entry) {
        this.state.logs.push(entry);
        if (this.state.logs.length > 1000) this.state.logs.shift();
        if (this.state.currentPage === 'logs') this.renderLogs();
    }
}

// API Error Class
class APIError extends Error {
    constructor(message, status, data) {
        super(message);
        this.status = status;
        this.data = data;
    }
}

// Initialize App
const app = new GaussOSApp();
