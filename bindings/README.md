# GaussTwin Language Bindings

This directory contains Python and TypeScript bindings for the GaussTwin framework.

## Prerequisites

- Rust toolchain (1.70 or later)
- Python 3.8 or later with pip
- Node.js and npm (for TypeScript/WASM builds)
- Deno (for TypeScript/Deno builds)

## Building the Bindings

The easiest way to build both bindings is to use the provided build script:

```bash
./build_bindings.sh
```

### Manual Build Instructions

#### Python Bindings

1. Install maturin:
   ```bash
   pip install maturin
   ```

2. Build and install:
   ```bash
   cd bindings/gausstwin-py
   maturin develop
   ```

#### TypeScript Bindings

1. Install wasm-pack:
   ```bash
   npm install -g wasm-pack
   ```

2. Build WASM package:
   ```bash
   cd bindings/gausstwin-ts
   wasm-pack build --target web
   ```

3. Build Deno plugin:
   ```bash
   cd bindings/gausstwin-ts
   cargo build --features deno --target wasm32-unknown-unknown
   ```

## Usage Examples

### Python

```python
import gausstwin as gt

# Create a simulation
model = gt.PyModel()
space = gt.PySpace([100.0, 100.0])
agent = gt.PyAgent("agent_1")

# Configure agent behavior
behavior = gt.PyBehavior("random_walk")
agent.set_behavior(behavior)

# Run simulation
await model.run(100)
```

### TypeScript (Web)

```typescript
import { TSModel, TSSpace, TSAgent, TSBehavior } from "gausstwin";

// Create a simulation
const model = new TSModel();
const space = new TSSpace([100, 100]);
const agent = new TSAgent("agent_1");

// Configure agent behavior
const behavior = new TSBehavior("random_walk");
agent.setBehavior(behavior);

// Run simulation
await model.run(100);
```

### TypeScript (Deno)

```typescript
import { Deno as GaussTwin } from "gausstwin";

// Initialize GaussTwin
await GaussTwin.initialize();

// Create a simulation
const model = GaussTwin.modelNew();
const space = GaussTwin.spaceNew([100, 100]);
const agent = GaussTwin.agentNew("agent_1");

// Configure agent behavior
const behavior = GaussTwin.behaviorNew("random_walk");
agent.setBehavior(behavior);

// Run simulation
await model.run(100);
```

## Features

Both bindings provide access to:

- Agent-based modeling
- Spatial operations
- Grid-based computations
- Neural network integration
- Performance metrics
- Visualization tools

## Examples

Check out the example files in each binding directory:

- Python: `gausstwin-py/examples/`
  - `basic_simulation.py`
  - `flocking_simulation.py`
  - `spatial_query.py`

- TypeScript: `gausstwin-ts/examples/`
  - `basic_simulation.ts`
  - `flocking_simulation.ts`
  - `spatial_query.ts`

## Documentation

For detailed API documentation, see:
- Python: `gausstwin-py/docs/`
- TypeScript: `gausstwin-ts/docs/`

## Troubleshooting

### Common Issues

1. **Python binding build fails**
   - Ensure you have Python development headers installed
   - Check Python version compatibility (3.8+)
   - Verify numpy is installed

2. **TypeScript/WASM build fails**
   - Ensure wasm-pack is installed
   - Check Node.js/npm versions
   - Verify you have wasm32-unknown-unknown target installed

3. **Runtime errors**
   - Check version compatibility between core and bindings
   - Verify all dependencies are installed
   - Enable debug logging for more information

### Getting Help

- File an issue on GitHub
- Check existing issues for solutions
- Consult the documentation
- Contact the maintainers 