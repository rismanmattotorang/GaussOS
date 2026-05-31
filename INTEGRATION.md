# GaussOS Backend-Frontend Integration Guide

## Overview

This guide provides comprehensive instructions for setting up and testing the integration between GaussOS backend services (Rust/Axum) and the Web-UI frontend (Deno/TypeScript).

## Architecture

```
┌─────────────────┐    HTTP/WebSocket    ┌─────────────────┐
│   Frontend      │ ◄──────────────────► │    Backend      │
│   (Deno/TS)     │                      │   (Rust/Axum)   │
│   Port: 3000    │                      │   Port: 8080    │
└─────────────────┘                      └─────────────────┘
        │                                         │
        │                                         │
        ▼                                         ▼
┌─────────────────┐                      ┌─────────────────┐
│   Static Files  │                      │   Database      │
│   (CSS, JS)     │                      │   (PostgreSQL)  │
└─────────────────┘                      └─────────────────┘
```

## Quick Start

### 1. Start Backend Services

```bash
# From project root
cargo run --bin gaussos server --host 0.0.0.0 --port 8080
```

### 2. Start Frontend Services

```bash
# From project root
cd web-ui
./start.sh
```

### 3. Test Integration

```bash
# From project root
./scripts/test-integration.sh
```

## API Endpoints

### Backend API (Port 8080)

| Endpoint | Method | Description | Response |
|----------|--------|-------------|----------|
| `/health` | GET | System health check | `{"status": "healthy", "version": "2.1.0"}` |
| `/metrics` | GET | System metrics | `{"memory_operations_total": 1000, ...}` |
| `/api/v1/memories` | GET | List memories | `[{"id": "...", "content": "..."}]` |
| `/api/v1/memories` | POST | Create memory | `{"id": "...", "message": "..."}` |
| `/api/v1/memories/:id` | GET | Get memory | `{"id": "...", "content": "..."}` |
| `/api/v1/memories/search` | POST | Search memories | `[{"id": "...", "content": "..."}]` |
| `/api/v1/admin/stats` | GET | System statistics | `{"database": {...}, "system": {...}}` |
| `/api/v1/admin/backup` | POST | Create backup | `{"backup_id": "...", "status": "..."}` |
| `/api/v1/admin/optimize` | POST | Optimize system | `{"success": true, "improvements": {...}}` |

### Frontend API (Port 3000)

| Endpoint | Method | Description | Response |
|----------|--------|-------------|----------|
| `/` | GET | Main dashboard | HTML page |
| `/api/status` | GET | Proxied backend status | Backend response |
| `/api/metrics` | GET | Proxied backend metrics | Backend response |
| `/api/optimize` | GET | Proxied backend optimization | Backend response |
| `/static/*` | GET | Static assets | CSS, JS, images |

## Configuration

### Backend Configuration

The backend can be configured via environment variables or configuration files:

```bash
# Environment variables
export GAUSSOS_HOST=0.0.0.0
export GAUSSOS_PORT=8080
export GAUSSOS_DATABASE_URL=postgresql://user:pass@localhost/gaussos
export GAUSSOS_JWT_SECRET=your-secret-key

# Or via configuration file
cargo run --bin gaussos server --config config.toml
```

### Frontend Configuration

The frontend API client is configured in `web-ui/api-client.ts`:

```typescript
export const apiClient = new ApiClient({
    baseUrl: 'http://localhost:8080',  // Backend URL
    timeout: 10000,                    // Request timeout
});
```

## Integration Features

### 1. API Client

The frontend includes a comprehensive API client (`web-ui/api-client.ts`) that provides:

- **Type-safe API calls** with proper error handling
- **Automatic retries** with exponential backoff
- **Request/response logging** for debugging
- **Fallback mechanisms** when backend is unavailable

### 2. CORS Configuration

The backend is configured with permissive CORS settings for development:

```rust
CorsLayer::new()
    .allow_origin(Any)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([
        HeaderName::from_static("content-type"),
        HeaderName::from_static("authorization"),
        HeaderName::from_static("x-api-key"),
    ])
```

### 3. Error Handling

Both frontend and backend implement comprehensive error handling:

- **HTTP status codes** for different error types
- **Structured error responses** with details
- **Graceful degradation** when services are unavailable
- **User-friendly error messages** in the UI

### 4. Real-time Updates

The system supports real-time updates through:

- **WebSocket connections** for live data
- **Server-sent events** for notifications
- **Polling fallback** when real-time is unavailable

## Testing

### Manual Testing

1. **Start both services** as described in Quick Start
2. **Open browser** to `http://localhost:3000`
3. **Navigate through** different sections of the dashboard
4. **Check browser console** for any errors
5. **Verify API calls** in Network tab

### Automated Testing

Run the integration test suite:

```bash
./scripts/test-integration.sh
```

This script tests:
- ✅ Service availability
- ✅ API endpoint responses
- ✅ CORS configuration
- ✅ Frontend-backend communication
- ✅ Error handling

### Load Testing

Test system performance under load:

```bash
# Test backend API
cargo run --bin gaussos test performance

# Test frontend responsiveness
cd web-ui && deno test --allow-net
```

## Troubleshooting

### Common Issues

#### 1. Backend Not Starting

```bash
# Check if port is in use
lsof -i :8080

# Check logs
cargo run --bin gaussos server --verbose

# Check database connection
cargo run --bin gaussos database health
```

#### 2. Frontend Not Starting

```bash
# Check if Deno is installed
deno --version

# Check if port is in use
lsof -i :3000

# Check logs
cd web-ui && tail -f logs/frontend.log
```

#### 3. CORS Errors

If you see CORS errors in browser console:

1. **Verify backend CORS configuration** in `src/api/mod.rs`
2. **Check frontend API client** base URL in `web-ui/api-client.ts`
3. **Ensure both services** are running on correct ports

#### 4. API Timeout Errors

If API calls are timing out:

1. **Check network connectivity** between frontend and backend
2. **Verify backend is responding** with `curl http://localhost:8080/health`
3. **Increase timeout** in `web-ui/api-client.ts`
4. **Check backend logs** for slow queries

### Debug Mode

Enable debug logging:

```bash
# Backend debug
RUST_LOG=debug cargo run --bin gaussos server

# Frontend debug
cd web-ui && deno run --allow-net --allow-read main.ts --debug
```

## Performance Optimization

### Backend Optimizations

1. **Connection pooling** for database connections
2. **Response compression** for large payloads
3. **Caching layers** (L1/L2/L3) for frequently accessed data
4. **SIMD acceleration** for vector operations

### Frontend Optimizations

1. **Static asset caching** with proper headers
2. **Code splitting** for large JavaScript bundles
3. **Lazy loading** for non-critical components
4. **Request batching** for multiple API calls

## Security Considerations

### Production Deployment

1. **HTTPS only** - Configure TLS certificates
2. **CORS restrictions** - Limit allowed origins
3. **Rate limiting** - Prevent abuse
4. **Authentication** - Implement proper auth flow
5. **Input validation** - Sanitize all inputs

### Development Security

1. **Local development** - Use localhost only
2. **No sensitive data** - Use mock data for development
3. **Secure defaults** - Safe configuration defaults

## Monitoring

### Health Checks

Monitor service health:

```bash
# Backend health
curl http://localhost:8080/health

# Frontend health
curl http://localhost:3000/api/status

# Integration health
./scripts/test-integration.sh
```

### Metrics

Track system performance:

```bash
# Backend metrics
curl http://localhost:8080/metrics

# System statistics
curl http://localhost:8080/api/v1/admin/stats
```

## Development Workflow

### 1. Backend Development

```bash
# Start backend in development mode
cargo run --bin gaussos server --host 0.0.0.0 --port 8080

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt
```

### 2. Frontend Development

```bash
# Start frontend in development mode
cd web-ui
deno run --allow-net --allow-read main.ts

# Run tests
deno test --allow-net

# Check code quality
deno lint
deno fmt
```

### 3. Integration Development

```bash
# Start both services
./scripts/start.sh

# Run integration tests
./scripts/test-integration.sh

# Monitor logs
tail -f logs/*.log
```

## Conclusion

The GaussOS backend-frontend integration provides a robust, scalable foundation for AI memory management. The system is designed for both development and production use with comprehensive testing, monitoring, and security features.

For additional support or questions, refer to the main documentation or create an issue in the project repository.
