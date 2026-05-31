# GaussOS Management Scripts

Professional, beautiful, and comprehensive management scripts for the GaussOS platform.

## 🚀 Quick Start

```bash
# Make scripts executable (if not already done)
chmod +x scripts/*.sh

# Start the main management interface
./scripts/manage.sh

# Or use direct commands
./scripts/manage.sh start
./scripts/manage.sh status
./scripts/manage.sh monitor
```

## 📋 Available Scripts

### 🔧 **Build & Development Scripts**

#### 1. `compile.sh` - Comprehensive Build System
**Advanced compilation system for backend and frontend components**

**Features:**
- Complete Rust backend compilation (debug + release)
- TypeScript frontend compilation and bundling
- Code quality checks (clippy, formatting)
- Documentation generation
- Development environment setup
- Build optimization analysis

**Usage:**
```bash
# Full compilation
./scripts/compile.sh

# Clean build
./scripts/compile.sh --clean

# Backend only
./scripts/compile.sh --skip-frontend

# Frontend only
./scripts/compile.sh --skip-backend

# Setup development environment
./scripts/compile.sh --setup-dev

# Show help
./scripts/compile.sh --help
```

#### 2. `test.sh` - Comprehensive Testing Suite
**Enterprise-grade testing with coverage and validation**

**Features:**
- Unit testing with coverage analysis
- Integration testing (backend + frontend)
- Performance validation
- Code quality linting
- Frontend TypeScript validation
- Test report generation

**Usage:**
```bash
# Run all tests
./scripts/test.sh

# Skip frontend tests
./scripts/test.sh --skip-frontend

# Skip backend tests
./scripts/test.sh --skip-backend

# Cleanup after tests
./scripts/test.sh --cleanup

# Show help
./scripts/test.sh --help
```

#### 3. `bench.sh` - Performance Benchmarking
**Advanced performance testing and profiling system**

**Features:**
- Rust benchmark execution
- Frontend performance testing
- System profiling with perf/valgrind
- Load testing scenarios
- Performance comparison reports
- System monitoring during tests

**Usage:**
```bash
# Run all benchmarks
./scripts/bench.sh

# With system monitoring
./scripts/bench.sh --monitor

# Skip profiling
./scripts/bench.sh --skip-profiling

# Archive results
./scripts/bench.sh --archive

# Show help
./scripts/bench.sh --help
```

### 🚀 **Service Management Scripts**

#### 4. `manage.sh` - Main Management Interface
**Unified control center for all GaussOS operations**

**Features:**
- Interactive menu system with beautiful UI
- Direct command execution
- Service management (start/stop/restart)
- Database operations
- Real-time monitoring
- Performance analysis
- System maintenance

**Usage:**
```bash
# Interactive mode
./scripts/manage.sh

# Direct commands
./scripts/manage.sh start          # Start all services
./scripts/manage.sh stop           # Stop all services
./scripts/manage.sh status         # Check system status
./scripts/manage.sh monitor        # Start real-time dashboard
./scripts/manage.sh performance    # Performance analysis
./scripts/manage.sh alerts         # System alerts
```

### 2. `start.sh` - Service Management
**Comprehensive service lifecycle management**

**Features:**
- Start/stop/restart backend and frontend services
- Automatic service health checks
- Log management and monitoring
- Build and test automation
- System cleanup utilities

**Usage:**
```bash
./scripts/start.sh start           # Start all services
./scripts/start.sh stop            # Stop all services
./scripts/start.sh restart         # Restart all services
./scripts/start.sh status          # Check service status
./scripts/start.sh logs backend    # View backend logs
./scripts/start.sh logs frontend   # View frontend logs
./scripts/start.sh build           # Build project
./scripts/start.sh test            # Run tests
./scripts/start.sh clean           # Clean system
```

### 3. `database.sh` - Database Management
**Professional database administration tools**

**Features:**
- Database health monitoring
- Automated backup and restore
- Performance optimization
- Backup management and cleanup
- Statistics and metrics

**Usage:**
```bash
./scripts/database.sh health       # Check database health
./scripts/database.sh backup       # Create backup
./scripts/database.sh restore      # Restore from backup
./scripts/database.sh list         # List available backups
./scripts/database.sh optimize     # Optimize database
./scripts/database.sh stats        # Show statistics
./scripts/database.sh clean        # Clean old backups
```

### 4. `monitor.sh` - Monitoring & Performance
**Real-time system monitoring and performance analysis**

**Features:**
- Real-time dashboard with live metrics
- Performance analysis and recommendations
- System alerts and notifications
- Metrics export (JSON/CSV)
- Resource usage monitoring

**Usage:**
```bash
./scripts/monitor.sh dashboard     # Real-time dashboard
./scripts/monitor.sh performance   # Performance analysis
./scripts/monitor.sh alerts        # System alerts
./scripts/monitor.sh export json   # Export metrics as JSON
./scripts/monitor.sh export csv    # Export metrics as CSV
```

## 🎨 Beautiful Console UI

All scripts feature:
- **Color-coded status indicators** (✓ ✗ ⚠)
- **Unicode symbols and emojis** for visual appeal
- **Professional formatting** with borders and sections
- **Real-time updates** and progress indicators
- **Interactive menus** with clear navigation

## 🔧 Configuration

### Environment Variables
```bash
# Optional: Override default ports
export GAUSSOS_BACKEND_PORT=8080
export GAUSSOS_FRONTEND_PORT=3000

# Optional: Override log directories
export GAUSSOS_LOG_DIR=./logs
export GAUSSOS_BACKUP_DIR=./backups
```

### Configuration Files
- `config.toml` - Main GaussOS configuration
- `scripts/config.sh` - Script-specific settings (if needed)

## 📊 Monitoring Dashboard

### Real-time Metrics
- **System Resources**: CPU, Memory, Disk usage
- **Service Status**: Backend, Frontend, Database health
- **Performance Metrics**: Response times, throughput, cache hit rates
- **API Statistics**: Request rates, error rates, active connections

### Performance Analysis
- **Cache Performance**: Hit rates and optimization recommendations
- **Response Times**: Average and percentile analysis
- **Throughput**: Requests per second and bottlenecks
- **Resource Usage**: Memory and CPU utilization patterns

## 🛡️ System Alerts

Automatic monitoring for:
- **High CPU usage** (>80%)
- **High memory usage** (>90%)
- **High disk usage** (>85%)
- **Service failures** and health check failures
- **Performance degradation** warnings

## 📦 Backup & Recovery

### Automated Backups
- **Scheduled backups** with timestamp naming
- **Compressed storage** to save space
- **Backup verification** and integrity checks
- **Retention policies** for old backups

### Recovery Procedures
- **One-click restore** from backup files
- **Backup validation** before restore
- **Rollback capabilities** for failed restores
- **Backup catalog** with metadata

## 🚀 Development Workflow

### Quick Development Commands
```bash
# Build and test
./scripts/manage.sh build
./scripts/manage.sh test

# Run demo
./scripts/manage.sh demo

# Start development environment
./scripts/manage.sh start
./scripts/manage.sh monitor
```

### Continuous Integration
```bash
# Automated testing
./scripts/start.sh test

# Performance benchmarking
cargo bench

# Code quality checks
cargo clippy
cargo fmt --check
```

## 🌐 Web Management Interface

Access the web-based management console at:
```
http://localhost:3000
```

**Features:**
- **Real-time dashboard** with live metrics
- **Service management** controls
- **Log viewing** and monitoring
- **Metrics export** functionality
- **Responsive design** for mobile devices

## 📈 Performance Optimization

### Built-in Optimizations
- **SIMD acceleration** for vector operations
- **Lock-free data structures** for high concurrency
- **Tiered caching** (L1/L2/L3) with ARC strategy
- **Batch processing** for improved throughput
- **Connection pooling** for database operations

### Monitoring Tools
- **Real-time performance tracking**
- **Bottleneck identification**
- **Resource usage analysis**
- **Optimization recommendations**

## 🔒 Security Features

- **Authentication** and authorization checks
- **Rate limiting** and DDoS protection
- **Secure backup** encryption (optional)
- **Audit logging** for all operations
- **Input validation** and sanitization

## 📝 Logging & Debugging

### Log Management
```bash
# View real-time logs
./scripts/start.sh logs backend
./scripts/start.sh logs frontend

# Export logs for analysis
./scripts/monitor.sh export json logs.json
```

### Debugging Tools
- **Detailed error messages** with context
- **Stack traces** for debugging
- **Performance profiling** data
- **Memory leak detection**

## 🏗️ Architecture

```
GaussOS Management Suite
├── manage.sh          # Main control interface
├── start.sh           # Service lifecycle management
├── database.sh        # Database administration
├── monitor.sh         # Monitoring and performance
└── web-ui/            # Web management interface
    ├── main.ts        # Web frontend
    └── static/        # Static assets
```

## 🎯 Best Practices

### Production Deployment
1. **Build optimized binaries**: `./scripts/manage.sh build`
2. **Configure environment**: Set production variables
3. **Start services**: `./scripts/manage.sh start`
4. **Monitor health**: `./scripts/manage.sh monitor`
5. **Set up backups**: `./scripts/database.sh backup`

### Development Workflow
1. **Start development**: `./scripts/manage.sh start`
2. **Monitor performance**: `./scripts/monitor.sh dashboard`
3. **Run tests**: `./scripts/manage.sh test`
4. **Check alerts**: `./scripts/monitor.sh alerts`

### Maintenance
1. **Regular backups**: Schedule automated backups
2. **Performance monitoring**: Monitor key metrics
3. **Log rotation**: Manage log file sizes
4. **System cleanup**: Remove old files and backups

## 🆘 Troubleshooting

### Common Issues

**Service won't start:**
```bash
# Check logs
./scripts/start.sh logs backend

# Verify configuration
cat config.toml

# Check dependencies
cargo check
```

**High memory usage:**
```bash
# Check performance
./scripts/monitor.sh performance

# Optimize database
./scripts/database.sh optimize

# Check for memory leaks
./scripts/monitor.sh alerts
```

**Database connection issues:**
```bash
# Check database health
./scripts/database.sh health

# Verify configuration
grep -i database config.toml

# Test connectivity
curl http://localhost:8080/health
```

### Getting Help
- **Check logs**: All scripts provide detailed logging
- **Monitor dashboard**: Real-time system status
- **Performance analysis**: Identify bottlenecks
- **System alerts**: Automatic problem detection

## 📚 Additional Resources

- **README.md**: Main project documentation
- **ARCHITECTURE.md**: System architecture details
- **SPECS.md**: Technical specifications
- **TUTORIAL.md**: Step-by-step tutorials
- **API Documentation**: REST API reference

---

**GaussOS Management Suite** - Professional, beautiful, and comprehensive management tools for the advanced AI memory management platform.
