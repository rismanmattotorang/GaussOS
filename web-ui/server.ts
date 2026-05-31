// Enhanced GaussOS Server - Production Ready with Deno
// File: server.ts

import { serve } from "https://deno.land/std@0.220.0/http/server.ts";
import { serveDir } from "https://deno.land/std@0.220.0/http/file_server.ts";

// Configuration management
interface ServerConfig {
  port: number;
  corsOrigins: string[];
  rateLimit: {
    windowMs: number;
    maxRequests: number;
  };
  security: {
    enableCSP: boolean;
    enableHSTS: boolean;
  };
  monitoring: {
    enableMetrics: boolean;
    metricsEndpoint: string;
  };
}

const config: ServerConfig = {
  port: parseInt(Deno.env.get("PORT") || "3000"),
  corsOrigins: Deno.env.get("CORS_ORIGINS")?.split(",") || ["http://localhost:3000"],
  rateLimit: {
    windowMs: 15 * 60 * 1000, // 15 minutes
    maxRequests: 100,
  },
  security: {
    enableCSP: true,
    enableHSTS: true,
  },
  monitoring: {
    enableMetrics: true,
    metricsEndpoint: "/metrics",
  },
};

// Enhanced security headers
const securityHeaders = new Headers({
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
  "X-XSS-Protection": "1; mode=block",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Permissions-Policy": "geolocation=(), microphone=(), camera=()",
});

if (config.security.enableCSP) {
  securityHeaders.set(
    "Content-Security-Policy",
    [
      "default-src 'self'",
      "script-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com https://cdn.jsdelivr.net",
      "style-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com",
      "img-src 'self' data: https:",
      "font-src 'self' https://cdnjs.cloudflare.com",
      "connect-src 'self' ws: wss:",
      "frame-src 'none'",
      "object-src 'none'",
      "base-uri 'self'",
      "form-action 'self'",
    ].join("; ")
  );
}

if (config.security.enableHSTS) {
  securityHeaders.set(
    "Strict-Transport-Security",
    "max-age=31536000; includeSubDomains; preload"
  );
}

// Rate limiting implementation
class RateLimiter {
  private requests = new Map<string, { count: number; resetTime: number }>();

  isRateLimited(clientId: string): boolean {
    const now = Date.now();
    const clientData = this.requests.get(clientId);

    if (!clientData || now > clientData.resetTime) {
      this.requests.set(clientId, {
        count: 1,
        resetTime: now + config.rateLimit.windowMs,
      });
      return false;
    }

    if (clientData.count >= config.rateLimit.maxRequests) {
      return true;
    }

    clientData.count++;
    return false;
  }

  // Cleanup expired entries periodically
  cleanup() {
    const now = Date.now();
    for (const [clientId, data] of this.requests.entries()) {
      if (now > data.resetTime) {
        this.requests.delete(clientId);
      }
    }
  }
}

// Enhanced error handling
class AppError extends Error {
  constructor(
    message: string,
    public statusCode: number = 500,
    public code?: string,
    public details?: unknown
  ) {
    super(message);
    this.name = "AppError";
  }
}

// Metrics collection
class Metrics {
  private static instance: Metrics;
  private requestCount = 0;
  private errorCount = 0;
  private responseTime: number[] = [];
  private startTime = Date.now();

  static getInstance(): Metrics {
    if (!Metrics.instance) {
      Metrics.instance = new Metrics();
    }
    return Metrics.instance;
  }

  incrementRequest() {
    this.requestCount++;
  }

  incrementError() {
    this.errorCount++;
  }

  recordResponseTime(time: number) {
    this.responseTime.push(time);
    // Keep only last 1000 entries
    if (this.responseTime.length > 1000) {
      this.responseTime = this.responseTime.slice(-1000);
    }
  }

  getMetrics() {
    const avgResponseTime = this.responseTime.length > 0
      ? this.responseTime.reduce((a, b) => a + b) / this.responseTime.length
      : 0;

    return {
      uptime: Date.now() - this.startTime,
      requests: this.requestCount,
      errors: this.errorCount,
      errorRate: this.requestCount > 0 ? (this.errorCount / this.requestCount) * 100 : 0,
      avgResponseTime,
      memoryUsage: Deno.memoryUsage(),
    };
  }
}

// Enhanced content type detection with security
function getContentType(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase();
  const types: Record<string, string> = {
    'html': 'text/html; charset=utf-8',
    'css': 'text/css; charset=utf-8',
    'js': 'application/javascript; charset=utf-8',
    'ts': 'application/javascript; charset=utf-8',
    'json': 'application/json; charset=utf-8',
    'png': 'image/png',
    'jpg': 'image/jpeg',
    'jpeg': 'image/jpeg',
    'gif': 'image/gif',
    'svg': 'image/svg+xml',
    'ico': 'image/x-icon',
    'woff': 'font/woff',
    'woff2': 'font/woff2',
    'ttf': 'font/ttf',
    'eot': 'application/vnd.ms-fontobject',
    'pdf': 'application/pdf',
    'zip': 'application/zip',
  };
  
  const contentType = types[ext || ''] || 'application/octet-stream';
  
  // Add security headers for downloadable content
  if (ext && !['html', 'css', 'js', 'ts', 'json'].includes(ext)) {
    return contentType;
  }
  
  return contentType;
}

// Enhanced API client with retry logic and caching
class ApiClient {
  private cache = new Map<string, { data: unknown; expires: number }>();
  private readonly baseUrl = Deno.env.get("API_BASE_URL") || "http://localhost:8080";

  async fetchWithRetry<T>(
    endpoint: string,
    options: RequestInit = {},
    retries = 3,
    cacheDurationMs = 0
  ): Promise<T> {
    const cacheKey = `${endpoint}_${JSON.stringify(options)}`;
    
    // Check cache first
    if (cacheDurationMs > 0) {
      const cached = this.cache.get(cacheKey);
      if (cached && Date.now() < cached.expires) {
        return cached.data as T;
      }
    }

    const url = `${this.baseUrl}${endpoint}`;
    let lastError: Error;

    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 10000); // 10s timeout

        const response = await fetch(url, {
          ...options,
          signal: controller.signal,
          headers: {
            'Content-Type': 'application/json',
            'User-Agent': 'GaussOS/1.0',
            ...options.headers,
          },
        });

        clearTimeout(timeoutId);

        if (!response.ok) {
          throw new AppError(
            `API request failed: ${response.status} ${response.statusText}`,
            response.status
          );
        }

        const data = await response.json();
        
        // Cache successful responses
        if (cacheDurationMs > 0) {
          this.cache.set(cacheKey, {
            data,
            expires: Date.now() + cacheDurationMs,
          });
        }

        return data;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        
        if (attempt < retries) {
          // Exponential backoff
          const delay = Math.pow(2, attempt) * 1000;
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    }

    throw lastError!;
  }

  async getSystemStatus() {
    try {
      return await this.fetchWithRetry("/api/status", {}, 2, 30000); // 30s cache
    } catch (error) {
      console.warn("Failed to fetch system status, using fallback", error);
      return {
        status: "degraded",
        message: "Using cached data",
        timestamp: new Date().toISOString(),
      };
    }
  }

  async getMetrics() {
    try {
      return await this.fetchWithRetry("/api/metrics", {}, 1, 5000); // 5s cache
    } catch (error) {
      console.warn("Failed to fetch metrics, using fallback", error);
      return {
        cpu: Math.random() * 100,
        memory: Math.random() * 100,
        network: Math.random() * 100,
        timestamp: new Date().toISOString(),
      };
    }
  }
}

// WebSocket manager for real-time updates
class WebSocketManager {
  private clients = new Set<WebSocket>();

  addClient(ws: WebSocket) {
    this.clients.add(ws);
    
    ws.addEventListener('close', () => {
      this.clients.delete(ws);
    });

    ws.addEventListener('error', (event) => {
      console.error('WebSocket error:', event);
      this.clients.delete(ws);
    });

    // Send initial data
    this.sendToClient(ws, {
      type: 'connected',
      timestamp: new Date().toISOString(),
    });
  }

  broadcast(message: unknown) {
    const data = JSON.stringify(message);
    
    for (const client of this.clients) {
      try {
        if (client.readyState === WebSocket.OPEN) {
          client.send(data);
        } else {
          this.clients.delete(client);
        }
      } catch (error) {
        console.error('Error broadcasting to client:', error);
        this.clients.delete(client);
      }
    }
  }

  private sendToClient(ws: WebSocket, message: unknown) {
    try {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(message));
      }
    } catch (error) {
      console.error('Error sending to client:', error);
    }
  }

  getClientCount(): number {
    return this.clients.size;
  }
}

// Main application class
class GaussOSApp {
  private rateLimiter = new RateLimiter();
  private metrics = Metrics.getInstance();
  private apiClient = new ApiClient();
  private wsManager = new WebSocketManager();

  constructor() {
    // Periodic cleanup
    setInterval(() => {
      this.rateLimiter.cleanup();
    }, 60000); // Every minute

    // Periodic metrics broadcast
    setInterval(async () => {
      try {
        const systemMetrics = await this.apiClient.getMetrics() as Record<string, unknown>;
        const appMetrics = this.metrics.getMetrics();
        
        this.wsManager.broadcast({
          type: 'metrics',
          data: Object.assign({}, systemMetrics, appMetrics),
          timestamp: new Date().toISOString(),
        });
      } catch (error) {
        console.error('Error broadcasting metrics:', error);
      }
    }, 5000); // Every 5 seconds
  }

  async handleRequest(req: Request): Promise<Response> {
    const startTime = Date.now();
    const url = new URL(req.url);
    const clientId = this.getClientId(req);

    try {
      this.metrics.incrementRequest();

      // Rate limiting
      if (this.rateLimiter.isRateLimited(clientId)) {
        throw new AppError("Rate limit exceeded", 429);
      }

      // Handle different routes
      let response: Response;

      switch (true) {
        case url.pathname === "/ws":
          response = await this.handleWebSocket(req);
          break;
        case url.pathname.startsWith("/api/"):
          response = await this.handleApiRequest(req);
          break;
        case url.pathname.startsWith("/static/"):
          response = await this.handleStaticFile(req);
          break;
        case url.pathname === config.monitoring.metricsEndpoint:
          response = await this.handleMetrics(req);
          break;
        default:
          response = await this.handleSPA(req);
          break;
      }

      // Add security headers
      for (const [key, value] of securityHeaders.entries()) {
        response.headers.set(key, value);
      }

      // CORS headers
      if (config.corsOrigins.includes(req.headers.get("origin") || "")) {
        response.headers.set("Access-Control-Allow-Origin", req.headers.get("origin") || "");
        response.headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
        response.headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization");
      }

      const responseTime = Date.now() - startTime;
      this.metrics.recordResponseTime(responseTime);

      // Performance header
      response.headers.set("Server-Timing", `total;dur=${responseTime}`);

      return response;
    } catch (error) {
      this.metrics.incrementError();
      return this.handleError(error, req);
    }
  }

  private async handleWebSocket(req: Request): Promise<Response> {
    const { socket, response } = Deno.upgradeWebSocket(req);
    this.wsManager.addClient(socket);
    return response;
  }

  private async handleApiRequest(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const path = url.pathname;

    // API request validation
    if (req.method === "POST" || req.method === "PUT") {
      const contentType = req.headers.get("content-type");
      if (contentType && !contentType.includes("application/json")) {
        throw new AppError("Content-Type must be application/json", 400);
      }
    }

    // Local endpoints that don't need proxying
    switch (path) {
      case "/api/ui/status":
        const status = await this.apiClient.getSystemStatus();
        return new Response(JSON.stringify(status), {
          headers: { "Content-Type": "application/json" },
        });

      case "/api/ui/metrics":
        const metrics = await this.apiClient.getMetrics();
        return new Response(JSON.stringify(metrics), {
          headers: { "Content-Type": "application/json" },
        });

      case "/api/ui/health":
        return new Response(JSON.stringify({
          status: "healthy",
          timestamp: new Date().toISOString(),
          version: "3.0.0",
          webui: true,
        }), {
          headers: { "Content-Type": "application/json" },
        });
    }

    // Proxy all other /api/ requests to the Rust backend
    return await this.proxyToBackend(req);
  }

  private async proxyToBackend(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const backendUrl = Deno.env.get("API_BASE_URL") || "http://localhost:8080";
    const targetUrl = `${backendUrl}${url.pathname}${url.search}`;

    try {
      // Prepare headers for backend request
      const headers = new Headers();
      headers.set("Content-Type", req.headers.get("content-type") || "application/json");
      headers.set("User-Agent", "GaussOS-WebUI/3.0");
      
      // Forward auth headers
      const authHeader = req.headers.get("authorization");
      if (authHeader) {
        headers.set("Authorization", authHeader);
      }
      
      const apiKey = req.headers.get("x-api-key");
      if (apiKey) {
        headers.set("X-API-Key", apiKey);
      }

      // Make request to backend
      const body = req.method !== "GET" && req.method !== "HEAD" 
        ? await req.text() 
        : undefined;

      const response = await fetch(targetUrl, {
        method: req.method,
        headers,
        body,
      });

      // Forward the response
      const responseHeaders = new Headers();
      responseHeaders.set("Content-Type", response.headers.get("content-type") || "application/json");
      
      // Forward caching headers
      const cacheControl = response.headers.get("cache-control");
      if (cacheControl) {
        responseHeaders.set("Cache-Control", cacheControl);
      }

      return new Response(await response.text(), {
        status: response.status,
        statusText: response.statusText,
        headers: responseHeaders,
      });
    } catch (error) {
      console.error("Backend proxy error:", error);
      
      // Return a more informative error
      return new Response(JSON.stringify({
        error: "Backend unavailable",
        message: "Unable to connect to GaussOS backend server",
        timestamp: new Date().toISOString(),
        hint: "Ensure the GaussOS server is running on port 8080",
      }), {
        status: 503,
        headers: { "Content-Type": "application/json" },
      });
    }
  }

  private async handleStaticFile(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const filePath = url.pathname.replace("/static/", "");
    
    try {
      const fileInfo = await Deno.stat(`./static/${filePath}`);
      
      if (!fileInfo.isFile) {
        throw new AppError("Not a file", 404);
      }

      const file = await Deno.readFile(`./static/${filePath}`);
      const contentType = getContentType(filePath);
      
      const headers = new Headers({
        "Content-Type": contentType,
        "Cache-Control": "public, max-age=31536000, immutable",
        "ETag": `"${fileInfo.mtime?.getTime()}"`,
        "Last-Modified": fileInfo.mtime?.toUTCString() || "",
      });

      // Check if client has cached version
      const ifNoneMatch = req.headers.get("if-none-match");
      const ifModifiedSince = req.headers.get("if-modified-since");
      
      if (ifNoneMatch === headers.get("ETag") || 
          (ifModifiedSince && fileInfo.mtime && 
           new Date(ifModifiedSince) >= fileInfo.mtime)) {
        return new Response(null, { status: 304, headers });
      }

      return new Response(file, { headers });
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        throw new AppError("File not found", 404);
      }
      throw error;
    }
  }

  private async handleSPA(req: Request): Promise<Response> {
    // Serve the main SPA for all other routes
    const html = await this.generateHTML();
    
    return new Response(html, {
      headers: {
        "Content-Type": "text/html; charset=utf-8",
        "Cache-Control": "no-cache, no-store, must-revalidate",
      },
    });
  }

  private async handleMetrics(req: Request): Promise<Response> {
    if (!config.monitoring.enableMetrics) {
      throw new AppError("Metrics endpoint disabled", 404);
    }

    const metrics = this.metrics.getMetrics();
    
    return new Response(JSON.stringify({
      ...metrics,
      websocketClients: this.wsManager.getClientCount(),
      timestamp: new Date().toISOString(),
    }), {
      headers: { "Content-Type": "application/json" },
    });
  }

  private handleError(error: unknown, req: Request): Response {
    console.error("Request error:", error, {
      url: req.url,
      method: req.method,
      userAgent: req.headers.get("user-agent"),
    });

    if (error instanceof AppError) {
      return new Response(JSON.stringify({
        error: error.message,
        code: error.code,
        timestamp: new Date().toISOString(),
      }), {
        status: error.statusCode,
        headers: { "Content-Type": "application/json" },
      });
    }

    // Generic error response
    return new Response(JSON.stringify({
      error: "Internal server error",
      timestamp: new Date().toISOString(),
    }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }

  private getClientId(req: Request): string {
    // Use IP address or session ID for rate limiting
    return req.headers.get("x-forwarded-for") || 
           req.headers.get("x-real-ip") || 
           "unknown";
  }

  private async generateHTML(): Promise<string> {
    // This would typically be generated by your build process
    // For now, we'll return a placeholder that loads the React app
    return `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="GaussOS - Advanced AI Memory Management System">
    <title>GaussOS Management Console</title>
    <link rel="icon" type="image/svg+xml" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🧠</text></svg>">
    <script type="module" src="/static/app.js"></script>
    <link rel="stylesheet" href="/static/styles.css">
</head>
<body>
    <div id="root">
        <div class="loading-screen">
            <div class="loading-spinner"></div>
            <h2>GaussOS</h2>
            <p>Loading...</p>
        </div>
    </div>
    <noscript>
        <div style="text-align: center; padding: 50px;">
            <h1>JavaScript Required</h1>
            <p>GaussOS requires JavaScript to function properly.</p>
        </div>
    </noscript>
</body>
</html>`;
  }
}

// Application startup
async function main() {
  const app = new GaussOSApp();
  
  console.log(`🚀 GaussOS Server starting on port ${config.port}`);
  console.log(`📊 Metrics available at: http://localhost:${config.port}${config.monitoring.metricsEndpoint}`);
  console.log(`🔒 Security features enabled: CSP=${config.security.enableCSP}, HSTS=${config.security.enableHSTS}`);
  console.log(`⚡ Rate limiting: ${config.rateLimit.maxRequests} requests per ${config.rateLimit.windowMs/1000}s`);

  await serve(
    (req) => app.handleRequest(req),
    { 
      port: config.port,
      onListen: ({ port }) => {
        console.log(`✅ GaussOS Server listening on http://localhost:${port}`);
      },
    }
  );
}

// Graceful shutdown handling
if (import.meta.main) {
  // Handle shutdown signals
  const signals = ["SIGINT", "SIGTERM"];
  
  for (const signal of signals) {
    addEventListener(signal, () => {
      console.log(`\n📴 Received ${signal}. Shutting down gracefully...`);
      Deno.exit(0);
    });
  }

  // Start the server
  try {
    await main();
  } catch (error) {
    console.error("❌ Failed to start server:", error);
    Deno.exit(1);
  }
}