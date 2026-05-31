// API Client for GaussOS Backend Integration

export interface ApiConfig {
    baseUrl: string;
    timeout: number;
}

export class ApiClient {
    private config: ApiConfig;

    constructor(config: Partial<ApiConfig> = {}) {
        this.config = {
            baseUrl: config.baseUrl || 'http://localhost:8080',
            timeout: config.timeout || 10000,
        };
    }

    private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
        const url = `${this.config.baseUrl}${endpoint}`;
        
        const response = await fetch(url, {
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
            },
            ...options,
        });

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        return response.json();
    }

    // System endpoints
    async getSystemStatus() {
        return this.request('/health');
    }

    async getMetrics() {
        return this.request('/metrics');
    }

    async getSystemStats() {
        return this.request('/api/v1/admin/stats');
    }

    // Memory operations
    async listMemories(params: any = {}) {
        const searchParams = new URLSearchParams(params);
        const endpoint = `/api/v1/memories${searchParams.toString() ? `?${searchParams.toString()}` : ''}`;
        return this.request(endpoint);
    }

    async getMemory(id: string) {
        return this.request(`/api/v1/memories/${id}`);
    }

    async createMemory(memory: any) {
        return this.request('/api/v1/memories', {
            method: 'POST',
            body: JSON.stringify(memory),
        });
    }

    async searchMemories(query: any) {
        return this.request('/api/v1/memories/search', {
            method: 'POST',
            body: JSON.stringify(query),
        });
    }

    // Admin operations
    async createBackup() {
        return this.request('/api/v1/admin/backup', {
            method: 'POST',
        });
    }

    async optimizeSystem() {
        return this.request('/api/v1/admin/optimize', {
            method: 'POST',
        });
    }
}

// Global API client instance
export const apiClient = new ApiClient();
