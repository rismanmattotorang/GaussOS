// src/memory/manager/namespace.rs
//! Registry that maps namespace paths to `MemoryNamespace` structs.

use crate::core::MemoryNamespace;
use dashmap::DashMap;

#[derive(Debug, Default)]
pub struct Registry {
    inner: DashMap<String, MemoryNamespace>,
}

impl Registry {
    pub fn register(&self, ns: MemoryNamespace) {
        self.inner.insert(ns.to_string(), ns);
    }

    pub fn get(&self, path: &str) -> Option<MemoryNamespace> {
        self.inner.get(path).map(|v| v.clone())
    }

    pub fn list(&self) -> Vec<String> {
        self.inner.iter().map(|e| e.key().clone()).collect()
    }
}
