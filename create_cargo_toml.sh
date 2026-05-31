#!/bin/bash

# Function to create a basic Cargo.toml
create_cargo_toml() {
    local crate_path=$1
    local crate_name=$2
    local description=$3
    local dependencies=$4
    
    cat > "$crate_path/Cargo.toml" << EOF
[package]
name = "$crate_name"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }
repository = { workspace = true }
homepage = { workspace = true }
description = "$description"
authors = { workspace = true }
keywords = { workspace = true }
categories = { workspace = true }
rust-version = { workspace = true }

[dependencies]
$dependencies
EOF
}

# Core crates
create_cargo_toml "crates/gausstwin-io" "gausstwin-io" "I/O and serialization for GaussTwin" "
gausstwin-core = { path = \"../gausstwin-core\" }
serde = { workspace = true }
serde_json = { workspace = true }
arrow = { workspace = true }
parquet = { workspace = true }
bincode = { workspace = true }
tokio = { workspace = true }
"

create_cargo_toml "crates/gausstwin-visual" "gausstwin-visual" "Visualization server for GaussTwin" "
gausstwin-core = { path = \"../gausstwin-core\" }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
"

create_cargo_toml "crates/spaces-grid" "spaces-grid" "Grid-based spatial indexing" "
gausstwin-core = { path = \"../gausstwin-core\" }
ndarray = { workspace = true }
serde = { workspace = true }
"

create_cargo_toml "crates/spaces-continuous" "spaces-continuous" "Continuous space implementation" "
gausstwin-core = { path = \"../gausstwin-core\" }
nalgebra = { workspace = true }
rstar = { workspace = true }
kiddo = { workspace = true }
serde = { workspace = true }
"

create_cargo_toml "crates/spaces-graph" "spaces-graph" "Graph-based spatial topology" "
gausstwin-core = { path = \"../gausstwin-core\" }
petgraph = { workspace = true }
serde = { workspace = true }
"

create_cargo_toml "crates/pathfinding" "pathfinding" "Pathfinding algorithms" "
gausstwin-core = { path = \"../gausstwin-core\" }
spaces-grid = { path = \"../spaces-grid\" }
spaces-continuous = { path = \"../spaces-continuous\" }
spaces-graph = { path = \"../spaces-graph\" }
petgraph = { workspace = true }
"

create_cargo_toml "crates/gausstwin-des" "gausstwin-des" "Discrete-event simulation engine" "
gausstwin-core = { path = \"../gausstwin-core\" }
serde = { workspace = true }
tokio = { workspace = true }
"

create_cargo_toml "crates/gausstwin-fsm" "gausstwin-fsm" "Finite state machine engine" "
gausstwin-core = { path = \"../gausstwin-core\" }
serde = { workspace = true }
"

create_cargo_toml "crates/gausstwin-marl" "gausstwin-marl" "Multi-agent reinforcement learning" "
gausstwin-core = { path = \"../gausstwin-core\" }
ndarray = { workspace = true }
rand = { workspace = true }
serde = { workspace = true }
candle-core = { workspace = true }
candle-nn = { workspace = true }
"

create_cargo_toml "crates/gausstwin-llm" "gausstwin-llm" "vLLM foundational model integration" "
gausstwin-core = { path = \"../gausstwin-core\" }
pyo3 = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
"

create_cargo_toml "crates/gausstwin-vec" "gausstwin-vec" "Milvus vector database integration" "
gausstwin-core = { path = \"../gausstwin-core\" }
milvus = { workspace = true }
ndarray = { workspace = true }
serde = { workspace = true }
tokio = { workspace = true }
"

create_cargo_toml "crates/gausstwin-agent" "gausstwin-agent" "Intelligent agent framework" "
gausstwin-core = { path = \"../gausstwin-core\" }
gausstwin-llm = { path = \"../gausstwin-llm\" }
gausstwin-vec = { path = \"../gausstwin-vec\" }
serde = { workspace = true }
tokio = { workspace = true }
"

create_cargo_toml "crates/gausstwin-connectors" "gausstwin-connectors" "External system connectors" "
gausstwin-core = { path = \"../gausstwin-core\" }
tokio = { workspace = true }
serde = { workspace = true }
"

create_cargo_toml "crates/gausstwin-fmi" "gausstwin-fmi" "Functional Mock-up Interface integration" "
gausstwin-core = { path = \"../gausstwin-core\" }
serde = { workspace = true }
"

create_cargo_toml "crates/gausstwin-hla" "gausstwin-hla" "High Level Architecture interface" "
gausstwin-core = { path = \"../gausstwin-core\" }
serde = { workspace = true }
"

create_cargo_toml "crates/gausstwin-db" "gausstwin-db" "Database adapters for SurrealDB and Skytable" "
gausstwin-core = { path = \"../gausstwin-core\" }
surrealdb = { workspace = true }
skytable = { workspace = true }
serde = { workspace = true }
tokio = { workspace = true }
"

create_cargo_toml "crates/gausstwin-api" "gausstwin-api" "High-performance API server" "
gausstwin-core = { path = \"../gausstwin-core\" }
axum = { workspace = true }
tonic = { workspace = true }
prost = { workspace = true }
async-graphql = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
tokio = { workspace = true }
"

create_cargo_toml "crates/gausstwin-ml" "gausstwin-ml" "Machine learning utilities" "
gausstwin-core = { path = \"../gausstwin-core\" }
candle-core = { workspace = true }
candle-nn = { workspace = true }
ort = { workspace = true }
ndarray = { workspace = true }
serde = { workspace = true }
"

create_cargo_toml "bindings/gausstwin-py" "gausstwin-py" "Python bindings for GaussTwin" "
gausstwin-core = { path = \"../../crates/gausstwin-core\" }
pyo3 = { workspace = true }
serde = { workspace = true }

[lib]
name = \"gausstwin\"
crate-type = [\"cdylib\"]
"

create_cargo_toml "bindings/gausstwin-ts" "gausstwin-ts" "TypeScript/JavaScript bindings for GaussTwin" "
gausstwin-core = { path = \"../../crates/gausstwin-core\" }
wasm-bindgen = { workspace = true }
js-sys = { workspace = true }
web-sys = { workspace = true }
serde = { workspace = true }

[lib]
crate-type = [\"cdylib\"]
"

echo "All Cargo.toml files created successfully!" 