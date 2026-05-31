// src/memory/manager/cache.rs
//! Lock-free in-process cache for hot `MemCube`s.
//! Very small abstraction around `DashMap` so the outer manager can be refactored
//! without sprinkling `DashMap` calls everywhere.

use dashmap::{mapref::one::RefMut, DashMap};
use uuid::Uuid;

use crate::core::MemCube;

#[derive(Debug)]
pub struct MemoryCache {
    inner: DashMap<Uuid, MemCube>,
    capacity: usize,
}

impl MemoryCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: DashMap::new(),
            capacity,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn contains_key(&self, id: &Uuid) -> bool {
        self.inner.contains_key(id)
    }

    #[inline]
    pub fn insert(&self, id: Uuid, cube: MemCube) {
        if self.inner.len() >= self.capacity {
            // rudimentary eviction: random pop
            if let Some(entry) = self.inner.iter().next() {
                self.inner.remove(entry.key());
            }
        }
        self.inner.insert(id, cube);
    }

    #[inline]
    pub fn get(&self, id: &Uuid) -> Option<MemCube> {
        self.inner.get(id).map(|r| r.clone())
    }

    #[inline]
    pub fn get_mut(&self, id: &Uuid) -> Option<RefMut<'_, Uuid, MemCube>> {
        self.inner.get_mut(id)
    }

    #[inline]
    pub fn remove(&self, id: &Uuid) {
        self.inner.remove(id);
    }

    #[inline]
    pub fn iter(&self) -> dashmap::iter::Iter<'_, Uuid, MemCube> {
        self.inner.iter()
    }

    /// Remove expired entries (placeholder - MemCubes don't track TTL yet)
    pub fn cleanup_expired(&self) {
        // This is a no-op for now since MemCube doesn't have TTL
        // In a production implementation, we'd check timestamps and remove stale entries
        // For now, just enforce capacity if overflowed
        while self.inner.len() > self.capacity {
            if let Some(entry) = self.inner.iter().next() {
                self.inner.remove(entry.key());
            } else {
                break;
            }
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
