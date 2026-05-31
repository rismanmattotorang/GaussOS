// src/database/postgres.rs
//! PostgreSQL implementation for GaussOS memory storage
//! Provides enterprise-grade relational database capabilities with ACID compliance

use crate::{
    core::MemCube,
    database::{
        AgePercentiles, AgeStatistics, BackupConfig, BackupResult, DatabaseMetrics, MemVault,
        OptimizationResult, PerformanceMetrics, QualityDistribution, RealTimeMetrics,
        RestoreConfig, RestoreResult, SearchQuery, StorageMetrics, VaultStats,
    },
    error::{GaussOSError, Result},
};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{postgres::{PgPoolOptions, PgRow}, Pool, Postgres, Row};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

/// PostgreSQL vault implementation for enterprise memory storage
#[derive(Debug)]
pub struct PostgresVault {
    pool: Pool<Postgres>,
    connection_string: String,
}

impl PostgresVault {
    /// Create a new PostgreSQL vault with connection pooling
    pub async fn new(connection_string: &str) -> Result<Self> {
        info!("Connecting to PostgreSQL database");

        let pool = PgPoolOptions::new()
            .max_connections(20)
            // Fail fast when PostgreSQL is unreachable so the system can fall
            // back to other backends quickly instead of stalling on startup.
            .acquire_timeout(std::time::Duration::from_secs(3))
            .idle_timeout(std::time::Duration::from_secs(300))
            .max_lifetime(std::time::Duration::from_secs(1800))
            .connect(connection_string)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to connect to PostgreSQL: {}", e)))?;

        // Initialize database schema
        Self::initialize_schema(&pool).await?;

        Ok(Self {
            pool,
            connection_string: connection_string.to_string(),
        })
    }

    /// Initialize database schema with proper indexing
    async fn initialize_schema(pool: &Pool<Postgres>) -> Result<()> {
        let schema_sql = r#"
            CREATE TABLE IF NOT EXISTS memory_cubes (
                id UUID PRIMARY KEY,
                namespace VARCHAR(255) NOT NULL DEFAULT 'default',
                payload_type VARCHAR(50) NOT NULL,
                payload JSONB NOT NULL,
                metadata JSONB NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                version INTEGER NOT NULL DEFAULT 1,
                access_count BIGINT NOT NULL DEFAULT 0,
                last_accessed TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                ttl BIGINT,
                compression_level SMALLINT NOT NULL DEFAULT 0,
                quality_score DOUBLE PRECISION NOT NULL DEFAULT 1.0,
                priority VARCHAR(20) NOT NULL DEFAULT 'normal',
                tags TEXT[],
                relationships JSONB,
                provenance JSONB,
                schema_version VARCHAR(50),
                custom_attributes JSONB
            );

            -- Create indexes for performance
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_namespace ON memory_cubes(namespace);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_payload_type ON memory_cubes(payload_type);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_created_at ON memory_cubes(created_at);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_updated_at ON memory_cubes(updated_at);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_tags ON memory_cubes USING GIN(tags);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_payload_gin ON memory_cubes USING GIN(payload);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_metadata_gin ON memory_cubes USING GIN(metadata);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_priority ON memory_cubes(priority);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_quality_score ON memory_cubes(quality_score);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_access_count ON memory_cubes(access_count);
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_last_accessed ON memory_cubes(last_accessed);

            -- Create full-text search index
            CREATE INDEX IF NOT EXISTS idx_memory_cubes_fts ON memory_cubes USING GIN(
                to_tsvector('english', 
                    COALESCE(payload->>'content', '') || ' ' ||
                    COALESCE(metadata->>'name', '') || ' ' ||
                    COALESCE(metadata->>'description', '')
                )
            );
        "#;

        sqlx::query(schema_sql)
            .execute(pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to initialize schema: {}", e)))?;

        info!("PostgreSQL schema initialized successfully");
        Ok(())
    }

    /// Convert MemCube to database row
    fn memcube_to_row(memory: &MemCube) -> (Uuid, String, String, serde_json::Value, serde_json::Value, chrono::DateTime<Utc>, chrono::DateTime<Utc>, u32, u64, chrono::DateTime<Utc>, Option<u64>, u8, f64, String, Vec<String>, serde_json::Value, serde_json::Value, Option<String>, serde_json::Value) {
        let payload_type = match &memory.payload {
            crate::core::MemoryPayload::Text(_) => "text",
            crate::core::MemoryPayload::Plaintext { .. } => "plaintext",
            crate::core::MemoryPayload::Semantic { .. } => "semantic",
            crate::core::MemoryPayload::Episodic { .. } => "episodic",
            crate::core::MemoryPayload::Procedural { .. } => "procedural",
            crate::core::MemoryPayload::Parametric { .. } => "parametric",
            crate::core::MemoryPayload::Activation { .. } => "activation",
        };

        let tags: Vec<String> = memory.metadata.tags.clone();
        let relationships = serde_json::to_value(&memory.metadata.relationships).unwrap_or_default();
        let provenance = serde_json::to_value(&memory.metadata.provenance).unwrap_or_default();
        let custom_attributes = serde_json::to_value(&memory.metadata.custom_attributes).unwrap_or_default();

        (
            memory.id,
            memory.namespace.0.clone(),
            payload_type.to_string(),
            serde_json::to_value(&memory.payload).unwrap_or_default(),
            serde_json::to_value(&memory.metadata).unwrap_or_default(),
            memory.created_at,
            memory.updated_at,
            memory.version,
            memory.metadata.access_count,
            memory.metadata.last_accessed,
            memory.metadata.ttl,
            memory.metadata.compression_level,
            memory.metadata.quality_score,
            format!("{:?}", memory.metadata.priority).to_lowercase(),
            tags,
            relationships,
            provenance,
            memory.metadata.schema_version.clone(),
            custom_attributes,
        )
    }

    /// Convert database row to MemCube
    fn row_to_memcube(row: &PgRow) -> Result<MemCube> {
        let id: Uuid = row.try_get("id")?;
        let namespace: String = row.try_get("namespace")?;
        let payload_type: String = row.try_get("payload_type")?;
        let payload_json: serde_json::Value = row.try_get("payload")?;
        let metadata_json: serde_json::Value = row.try_get("metadata")?;
        let created_at: chrono::DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: chrono::DateTime<Utc> = row.try_get("updated_at")?;
        let version: i32 = row.try_get("version")?;
        let version = version as u32;

        let payload = serde_json::from_value(payload_json)
            .map_err(|e| GaussOSError::SerializationError(format!("Failed to deserialize payload: {}", e)))?;
        
        let metadata = serde_json::from_value(metadata_json)
            .map_err(|e| GaussOSError::SerializationError(format!("Failed to deserialize metadata: {}", e)))?;

        Ok(MemCube {
            id,
            metadata,
            payload,
            namespace: crate::core::MemoryNamespace(namespace),
            created_at,
            updated_at,
            version,
        })
    }
}

#[async_trait]
impl MemVault for PostgresVault {
    async fn store(&self, memory: &MemCube) -> Result<()> {
        let (id, namespace, payload_type, payload, metadata, created_at, updated_at, version, access_count, last_accessed, ttl, compression_level, quality_score, priority, tags, relationships, provenance, schema_version, custom_attributes) = Self::memcube_to_row(memory);

        let query = r#"
            INSERT INTO memory_cubes (
                id, namespace, payload_type, payload, metadata, created_at, updated_at, version,
                access_count, last_accessed, ttl, compression_level, quality_score, priority,
                tags, relationships, provenance, schema_version, custom_attributes
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            ON CONFLICT (id) DO UPDATE SET
                namespace = EXCLUDED.namespace,
                payload_type = EXCLUDED.payload_type,
                payload = EXCLUDED.payload,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at,
                version = EXCLUDED.version,
                access_count = EXCLUDED.access_count,
                last_accessed = EXCLUDED.last_accessed,
                ttl = EXCLUDED.ttl,
                compression_level = EXCLUDED.compression_level,
                quality_score = EXCLUDED.quality_score,
                priority = EXCLUDED.priority,
                tags = EXCLUDED.tags,
                relationships = EXCLUDED.relationships,
                provenance = EXCLUDED.provenance,
                schema_version = EXCLUDED.schema_version,
                custom_attributes = EXCLUDED.custom_attributes
        "#;

        sqlx::query(query)
            .bind(id)
            .bind(namespace)
            .bind(payload_type)
            .bind(payload)
            .bind(metadata)
            .bind(created_at)
            .bind(updated_at)
            .bind(version as i32)
            .bind(access_count as i64)
            .bind(last_accessed)
            .bind(ttl.map(|t| t as i64))
            .bind(compression_level as i16)
            .bind(quality_score)
            .bind(priority)
            .bind(&tags)
            .bind(relationships)
            .bind(provenance)
            .bind(schema_version)
            .bind(custom_attributes)
            .execute(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to store memory: {}", e)))?;

        info!("Stored memory {} in PostgreSQL", memory.id);
        Ok(())
    }

    async fn retrieve(&self, id: &Uuid) -> Result<Option<MemCube>> {
        let query = "SELECT * FROM memory_cubes WHERE id = $1";
        
        let row = sqlx::query(query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to retrieve memory: {}", e)))?;

        match row {
            Some(row) => {
                // Update access count and last accessed
                let update_query = "UPDATE memory_cubes SET access_count = access_count + 1, last_accessed = NOW() WHERE id = $1";
                sqlx::query(update_query)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| GaussOSError::DatabaseError(format!("Failed to update access count: {}", e)))?;

                let memory = Self::row_to_memcube(&row)?;
                info!("Retrieved memory {} from PostgreSQL", id);
                Ok(Some(memory))
            }
            None => {
                info!("Memory {} not found in PostgreSQL", id);
                Ok(None)
            }
        }
    }

    async fn update(&self, memory: &MemCube) -> Result<()> {
        let mut updated_memory = memory.clone();
        updated_memory.updated_at = Utc::now();
        updated_memory.version += 1;

        self.store(&updated_memory).await
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let query = "DELETE FROM memory_cubes WHERE id = $1";
        
        let result = sqlx::query(query)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to delete memory: {}", e)))?;

        if result.rows_affected() > 0 {
            info!("Deleted memory {} from PostgreSQL", id);
            Ok(())
        } else {
            Err(GaussOSError::memory_not_found(*id))
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<MemCube>> {
        // Build a simple search query
        // For complex searches, we prioritize namespace + text search
        let limit = query.limit.unwrap_or(100) as i64;
        let offset = query.offset.unwrap_or(0) as i64;
        
        let rows = if let Some(namespace) = &query.namespace {
            if let Some(text) = &query.text {
                // Search by namespace and text
                let sql = r#"
                    SELECT * FROM memory_cubes 
                    WHERE namespace = $1 
                    AND to_tsvector('english', COALESCE(payload->>'content', '') || ' ' || COALESCE(metadata->>'name', '') || ' ' || COALESCE(metadata->>'description', '')) @@ plainto_tsquery('english', $2)
                    ORDER BY created_at DESC
                    LIMIT $3 OFFSET $4
                "#;
                sqlx::query(sql)
                    .bind(namespace)
                    .bind(text)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await
            } else {
                // Search by namespace only
                let sql = "SELECT * FROM memory_cubes WHERE namespace = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3";
                sqlx::query(sql)
                    .bind(namespace)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await
            }
        } else if let Some(text) = &query.text {
            // Search by text only
            let sql = r#"
                SELECT * FROM memory_cubes 
                WHERE to_tsvector('english', COALESCE(payload->>'content', '') || ' ' || COALESCE(metadata->>'name', '') || ' ' || COALESCE(metadata->>'description', '')) @@ plainto_tsquery('english', $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
            "#;
            sqlx::query(sql)
                .bind(text)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
        } else if !query.tags.is_empty() {
            // Search by tags
            let sql = "SELECT * FROM memory_cubes WHERE tags && $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3";
            sqlx::query(sql)
                .bind(&query.tags)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
        } else {
            // Default: return all with pagination
            let sql = "SELECT * FROM memory_cubes ORDER BY created_at DESC LIMIT $1 OFFSET $2";
            sqlx::query(sql)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
        }.map_err(|e| GaussOSError::DatabaseError(format!("Failed to search memories: {}", e)))?;
        
        let mut memories = Vec::new();
        for row in rows {
            let memory = Self::row_to_memcube(&row)?;
            memories.push(memory);
        }

        info!("Found {} memories in PostgreSQL search", memories.len());
        Ok(memories)
    }


    async fn list_by_tags(&self, tags: &[String]) -> Result<Vec<MemCube>> {
        let query = "SELECT * FROM memory_cubes WHERE tags && $1 ORDER BY created_at DESC";
        
        let rows = sqlx::query(query)
            .bind(tags)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to list memories by tags: {}", e)))?;

        let mut memories = Vec::new();
        for row in rows {
            let memory = Self::row_to_memcube(&row)?;
            memories.push(memory);
        }

        info!("Found {} memories with tags {:?} in PostgreSQL", memories.len(), tags);
        Ok(memories)
    }

    async fn get_stats(&self) -> Result<VaultStats> {
        let total_memories_query = "SELECT COUNT(*) as count FROM memory_cubes";
        let storage_size_query = "SELECT pg_total_relation_size('memory_cubes') as size";
        let avg_size_query = "SELECT AVG(pg_column_size(payload) + pg_column_size(metadata)) as avg_size FROM memory_cubes";

        let total_memories: i64 = sqlx::query_scalar(total_memories_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to get total memories count: {}", e)))?;

        let storage_size: i64 = sqlx::query_scalar(storage_size_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to get storage size: {}", e)))?;

        let avg_size: Option<f64> = sqlx::query_scalar(avg_size_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to get average size: {}", e)))?;

        Ok(VaultStats {
            total_memories: total_memories as u64,
            storage_size: storage_size as u64,
            average_memory_size: avg_size.unwrap_or(0.0),
            // Additional stats would be implemented here
            ..Default::default()
        })
    }

    async fn backup(&self, _backup_config: &BackupConfig) -> Result<BackupResult> {
        // Implement PostgreSQL backup using pg_dump or similar
        Err(GaussOSError::NotImplemented("PostgreSQL backup not yet implemented".into()))
    }

    async fn restore(&self, _restore_config: &RestoreConfig) -> Result<RestoreResult> {
        // Implement PostgreSQL restore using pg_restore or similar
        Err(GaussOSError::NotImplemented("PostgreSQL restore not yet implemented".into()))
    }

    async fn optimize(&self) -> Result<OptimizationResult> {
        // Run PostgreSQL maintenance operations
        let vacuum_query = "VACUUM ANALYZE memory_cubes";
        let reindex_query = "REINDEX TABLE memory_cubes";

        sqlx::query(vacuum_query)
            .execute(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to vacuum table: {}", e)))?;

        sqlx::query(reindex_query)
            .execute(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to reindex table: {}", e)))?;

        Ok(OptimizationResult {
            operations_performed: vec![
                crate::database::OptimizationOperation {
                    operation_type: "vacuum".to_string(),
                    target: "memory_cubes".to_string(),
                    result: "completed".to_string(),
                },
                crate::database::OptimizationOperation {
                    operation_type: "reindex".to_string(),
                    target: "memory_cubes_pkey".to_string(),
                    result: "completed".to_string(),
                },
            ],
            space_reclaimed_bytes: 0,
            performance_improvement_percent: 0.0,
            duration_ms: 0,
        })
    }

    async fn get_real_time_metrics(&self) -> Result<RealTimeMetrics> {
        // Get real-time PostgreSQL metrics
        let active_connections_query = "SELECT count(*) FROM pg_stat_activity WHERE state = 'active'";
        let cache_hit_ratio_query = "
            SELECT 
                round(100.0 * sum(heap_blks_hit) / (sum(heap_blks_hit) + sum(heap_blks_read)), 2) as hit_ratio
            FROM pg_statio_user_tables 
            WHERE schemaname = 'public'
        ";

        let active_connections: i64 = sqlx::query_scalar(active_connections_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to get active connections: {}", e)))?;

        let cache_hit_ratio: Option<f64> = sqlx::query_scalar(cache_hit_ratio_query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| GaussOSError::DatabaseError(format!("Failed to get cache hit ratio: {}", e)))?;

        Ok(RealTimeMetrics {
            timestamp: Utc::now(),
            operations_per_second: 0.0, // Would need to implement query rate tracking
            active_queries: active_connections as u32,
            slow_queries: 0,
            cache_hit_rate: cache_hit_ratio.unwrap_or(0.0),
            connection_utilization: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            disk_io_mb_per_sec: 0.0,
            network_io_mb_per_sec: 0.0,
        })
    }
}
