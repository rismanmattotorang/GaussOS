-- ===============================================
-- GaussOS SurrealDB Schema v2.0
-- Focus: Memory Cubes, Embeddings, Graph Relationships, Real-time Operations
-- ===============================================

-- Define namespace and database
DEFINE NAMESPACE gaussos;
USE NS gaussos;
DEFINE DATABASE memory_system;
USE DB memory_system;

-- ===============================================
-- MEMORY CUBES - Core Memory Storage
-- ===============================================

-- Main memory cubes table with advanced features
DEFINE TABLE memory_cubes SCHEMAFULL
    PERMISSIONS 
        FOR select, update, delete WHERE user_id = $auth.id OR $auth.admin = true
        FOR create WHERE $auth.id != NONE;

-- Memory cube fields
DEFINE FIELD id ON memory_cubes TYPE record<memory_cubes>;
DEFINE FIELD user_id ON memory_cubes TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD namespace ON memory_cubes TYPE string ASSERT $value != NONE;
DEFINE FIELD metadata ON memory_cubes TYPE object DEFAULT {};
DEFINE FIELD payload ON memory_cubes TYPE object ASSERT $value != NONE;
DEFINE FIELD payload_type ON memory_cubes TYPE string ASSERT $value != NONE;
DEFINE FIELD priority ON memory_cubes TYPE string VALUE $value OR "medium" ASSERT $value IN ["low", "medium", "high", "critical"];
DEFINE FIELD memory_type ON memory_cubes TYPE string VALUE $value OR "semantic" ASSERT $value IN ["semantic", "episodic", "procedural", "working", "core", "recall", "archival"];

-- Advanced memory features
DEFINE FIELD embeddings ON memory_cubes TYPE array<float> DEFAULT [];
DEFINE FIELD embedding_model ON memory_cubes TYPE string;
DEFINE FIELD similarity_threshold ON memory_cubes TYPE float DEFAULT 0.7;
DEFINE FIELD content_hash ON memory_cubes TYPE string;
DEFINE FIELD fingerprint ON memory_cubes TYPE string;

-- Size and compression
DEFINE FIELD size_bytes ON memory_cubes TYPE int DEFAULT 0;
DEFINE FIELD compression_level ON memory_cubes TYPE int DEFAULT 0 ASSERT $value >= 0 AND $value <= 9;
DEFINE FIELD is_compressed ON memory_cubes TYPE bool DEFAULT false;
DEFINE FIELD original_size ON memory_cubes TYPE int;

-- Access and lifecycle
DEFINE FIELD access_count ON memory_cubes TYPE int DEFAULT 0;
DEFINE FIELD last_accessed ON memory_cubes TYPE datetime;
DEFINE FIELD ttl_seconds ON memory_cubes TYPE int;
DEFINE FIELD expires_at ON memory_cubes TYPE datetime;
DEFINE FIELD is_archived ON memory_cubes TYPE bool DEFAULT false;
DEFINE FIELD archived_at ON memory_cubes TYPE datetime;

-- Quality and validation
DEFINE FIELD quality_score ON memory_cubes TYPE float DEFAULT 0.0 ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD validation_status ON memory_cubes TYPE string DEFAULT "pending" ASSERT $value IN ["pending", "validated", "rejected", "needs_review"];
DEFINE FIELD validation_metadata ON memory_cubes TYPE object DEFAULT {};

-- Timestamps
DEFINE FIELD created_at ON memory_cubes TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON memory_cubes TYPE datetime DEFAULT time::now();
DEFINE FIELD version ON memory_cubes TYPE int DEFAULT 1;

-- Tags and categorization
DEFINE FIELD tags ON memory_cubes TYPE array<string> DEFAULT [];
DEFINE FIELD categories ON memory_cubes TYPE array<string> DEFAULT [];

-- ===============================================
-- MEMORY RELATIONSHIPS - Graph Structure
-- ===============================================

DEFINE TABLE memory_relationships SCHEMAFULL
    PERMISSIONS 
        FOR select WHERE user_id = $auth.id OR $auth.admin = true
        FOR create, update, delete WHERE user_id = $auth.id OR $auth.admin = true;

DEFINE FIELD id ON memory_relationships TYPE record<memory_relationships>;
DEFINE FIELD user_id ON memory_relationships TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD from_memory ON memory_relationships TYPE record<memory_cubes> ASSERT $value != NONE;
DEFINE FIELD to_memory ON memory_relationships TYPE record<memory_cubes> ASSERT $value != NONE;
DEFINE FIELD relationship_type ON memory_relationships TYPE string ASSERT $value != NONE;
DEFINE FIELD strength ON memory_relationships TYPE float DEFAULT 1.0 ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD confidence ON memory_relationships TYPE float DEFAULT 0.5 ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD bidirectional ON memory_relationships TYPE bool DEFAULT false;
DEFINE FIELD metadata ON memory_relationships TYPE object DEFAULT {};
DEFINE FIELD created_at ON memory_relationships TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON memory_relationships TYPE datetime DEFAULT time::now();

-- ===============================================
-- MEMORY VERSIONS - Snapshot & Rollback
-- ===============================================

DEFINE TABLE memory_versions SCHEMAFULL
    PERMISSIONS 
        FOR select WHERE user_id = $auth.id OR $auth.admin = true
        FOR create WHERE user_id = $auth.id OR $auth.admin = true
        FOR delete WHERE $auth.admin = true;

DEFINE FIELD id ON memory_versions TYPE record<memory_versions>;
DEFINE FIELD memory_id ON memory_versions TYPE record<memory_cubes> ASSERT $value != NONE;
DEFINE FIELD user_id ON memory_versions TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD version_number ON memory_versions TYPE int ASSERT $value > 0;
DEFINE FIELD snapshot_data ON memory_versions TYPE object ASSERT $value != NONE;
DEFINE FIELD change_description ON memory_versions TYPE string;
DEFINE FIELD change_type ON memory_versions TYPE string ASSERT $value IN ["create", "update", "merge", "split", "evolve", "archive"];
DEFINE FIELD created_at ON memory_versions TYPE datetime DEFAULT time::now();
DEFINE FIELD size_bytes ON memory_versions TYPE int DEFAULT 0;

-- ===============================================
-- MEMORY TIERS - Advanced Memory Management
-- ===============================================

DEFINE TABLE memory_tiers SCHEMAFULL
    PERMISSIONS 
        FOR select WHERE user_id = $auth.id OR $auth.admin = true
        FOR create, update WHERE user_id = $auth.id OR $auth.admin = true;

DEFINE FIELD id ON memory_tiers TYPE record<memory_tiers>;
DEFINE FIELD memory_id ON memory_tiers TYPE record<memory_cubes> ASSERT $value != NONE;
DEFINE FIELD user_id ON memory_tiers TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD tier_type ON memory_tiers TYPE string ASSERT $value IN ["working", "core", "recall", "archival"];
DEFINE FIELD tier_config ON memory_tiers TYPE object DEFAULT {};
DEFINE FIELD promotion_criteria ON memory_tiers TYPE object DEFAULT {};
DEFINE FIELD demotion_criteria ON memory_tiers TYPE object DEFAULT {};
DEFINE FIELD last_tier_change ON memory_tiers TYPE datetime;
DEFINE FIELD created_at ON memory_tiers TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON memory_tiers TYPE datetime DEFAULT time::now();

-- ===============================================
-- MEMORY EMBEDDINGS - Vector Storage
-- ===============================================

DEFINE TABLE memory_embeddings SCHEMAFULL
    PERMISSIONS 
        FOR select WHERE user_id = $auth.id OR $auth.admin = true
        FOR create, update WHERE user_id = $auth.id OR $auth.admin = true;

DEFINE FIELD id ON memory_embeddings TYPE record<memory_embeddings>;
DEFINE FIELD memory_id ON memory_embeddings TYPE record<memory_cubes> ASSERT $value != NONE;
DEFINE FIELD user_id ON memory_embeddings TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD embedding_type ON memory_embeddings TYPE string ASSERT $value IN ["content", "metadata", "semantic", "visual", "audio"];
DEFINE FIELD vector ON memory_embeddings TYPE array<float> ASSERT array::len($value) > 0;
DEFINE FIELD dimensions ON memory_embeddings TYPE int ASSERT $value > 0;
DEFINE FIELD model_name ON memory_embeddings TYPE string;
DEFINE FIELD model_version ON memory_embeddings TYPE string;
DEFINE FIELD similarity_metric ON memory_embeddings TYPE string DEFAULT "cosine" ASSERT $value IN ["cosine", "euclidean", "manhattan", "dot_product"];
DEFINE FIELD created_at ON memory_embeddings TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON memory_embeddings TYPE datetime DEFAULT time::now();

-- ===============================================
-- MEMORY OPERATIONS - Advanced Operations Log
-- ===============================================

DEFINE TABLE memory_operations SCHEMAFULL
    PERMISSIONS 
        FOR select WHERE user_id = $auth.id OR $auth.admin = true
        FOR create WHERE user_id = $auth.id OR $auth.admin = true;

DEFINE FIELD id ON memory_operations TYPE record<memory_operations>;
DEFINE FIELD user_id ON memory_operations TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD operation_type ON memory_operations TYPE string ASSERT $value IN ["add", "update", "delete", "merge", "split", "evolve", "archive", "relate", "noop"];
DEFINE FIELD memory_ids ON memory_operations TYPE array<record<memory_cubes>>;
DEFINE FIELD operation_data ON memory_operations TYPE object DEFAULT {};
DEFINE FIELD result_data ON memory_operations TYPE object DEFAULT {};
DEFINE FIELD status ON memory_operations TYPE string DEFAULT "pending" ASSERT $value IN ["pending", "in_progress", "completed", "failed", "cancelled"];
DEFINE FIELD error_message ON memory_operations TYPE string;
DEFINE FIELD processing_time_ms ON memory_operations TYPE int;
DEFINE FIELD quality_metrics ON memory_operations TYPE object DEFAULT {};
DEFINE FIELD created_at ON memory_operations TYPE datetime DEFAULT time::now();
DEFINE FIELD completed_at ON memory_operations TYPE datetime;

-- ===============================================
-- MEMORY CONFLICTS - Conflict Detection & Resolution
-- ===============================================

DEFINE TABLE memory_conflicts SCHEMAFULL
    PERMISSIONS 
        FOR select WHERE user_id = $auth.id OR $auth.admin = true
        FOR create, update WHERE user_id = $auth.id OR $auth.admin = true;

DEFINE FIELD id ON memory_conflicts TYPE record<memory_conflicts>;
DEFINE FIELD user_id ON memory_conflicts TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD conflict_type ON memory_conflicts TYPE string ASSERT $value IN ["duplicate", "contradiction", "overlap", "inconsistency"];
DEFINE FIELD involved_memories ON memory_conflicts TYPE array<record<memory_cubes>>;
DEFINE FIELD confidence_score ON memory_conflicts TYPE float ASSERT $value >= 0.0 AND $value <= 1.0;
DEFINE FIELD resolution_strategy ON memory_conflicts TYPE string;
DEFINE FIELD resolution_data ON memory_conflicts TYPE object DEFAULT {};
DEFINE FIELD status ON memory_conflicts TYPE string DEFAULT "detected" ASSERT $value IN ["detected", "analyzing", "resolved", "ignored"];
DEFINE FIELD detected_at ON memory_conflicts TYPE datetime DEFAULT time::now();
DEFINE FIELD resolved_at ON memory_conflicts TYPE datetime;

-- ===============================================
-- REAL-TIME SUBSCRIPTIONS - Live Updates
-- ===============================================

DEFINE TABLE memory_subscriptions SCHEMAFULL
    PERMISSIONS 
        FOR select, create, update, delete WHERE user_id = $auth.id OR $auth.admin = true;

DEFINE FIELD id ON memory_subscriptions TYPE record<memory_subscriptions>;
DEFINE FIELD user_id ON memory_subscriptions TYPE record<users> ASSERT $value != NONE;
DEFINE FIELD subscription_type ON memory_subscriptions TYPE string ASSERT $value IN ["memory_changes", "relationship_changes", "quality_updates", "conflict_notifications"];
DEFINE FIELD filters ON memory_subscriptions TYPE object DEFAULT {};
DEFINE FIELD webhook_url ON memory_subscriptions TYPE string;
DEFINE FIELD is_active ON memory_subscriptions TYPE bool DEFAULT true;
DEFINE FIELD created_at ON memory_subscriptions TYPE datetime DEFAULT time::now();
DEFINE FIELD last_triggered ON memory_subscriptions TYPE datetime;

-- ===============================================
-- INDEXES FOR PERFORMANCE
-- ===============================================

-- Memory cubes indexes
DEFINE INDEX idx_memory_cubes_user_namespace ON memory_cubes FIELDS user_id, namespace;
DEFINE INDEX idx_memory_cubes_type ON memory_cubes FIELDS memory_type;
DEFINE INDEX idx_memory_cubes_tags ON memory_cubes FIELDS tags;
DEFINE INDEX idx_memory_cubes_created ON memory_cubes FIELDS created_at;
DEFINE INDEX idx_memory_cubes_quality ON memory_cubes FIELDS quality_score;
DEFINE INDEX idx_memory_cubes_access ON memory_cubes FIELDS last_accessed;
DEFINE INDEX idx_memory_cubes_hash ON memory_cubes FIELDS content_hash;

-- Embeddings indexes
DEFINE INDEX idx_embeddings_memory ON memory_embeddings FIELDS memory_id;
DEFINE INDEX idx_embeddings_type ON memory_embeddings FIELDS embedding_type;
DEFINE INDEX idx_embeddings_model ON memory_embeddings FIELDS model_name;

-- Relationships indexes
DEFINE INDEX idx_relationships_from ON memory_relationships FIELDS from_memory;
DEFINE INDEX idx_relationships_to ON memory_relationships FIELDS to_memory;
DEFINE INDEX idx_relationships_type ON memory_relationships FIELDS relationship_type;
DEFINE INDEX idx_relationships_strength ON memory_relationships FIELDS strength;

-- Operations indexes
DEFINE INDEX idx_operations_user_type ON memory_operations FIELDS user_id, operation_type;
DEFINE INDEX idx_operations_status ON memory_operations FIELDS status;
DEFINE INDEX idx_operations_created ON memory_operations FIELDS created_at;

-- ===============================================
-- FUNCTIONS FOR ADVANCED OPERATIONS
-- ===============================================

-- Vector similarity function
DEFINE FUNCTION fn::vector_similarity($vec1: array<float>, $vec2: array<float>, $metric: string) {
    RETURN IF $metric = "cosine" THEN {
        LET $dot = math::sum(array::zip($vec1, $vec2).{$value.0 * $value.1});
        LET $norm1 = math::sqrt(math::sum($vec1.{$value * $value}));
        LET $norm2 = math::sqrt(math::sum($vec2.{$value * $value}));
        RETURN $dot / ($norm1 * $norm2);
    } ELSE IF $metric = "euclidean" THEN {
        LET $diff = array::zip($vec1, $vec2).{$value.0 - $value.1};
        RETURN 1 / (1 + math::sqrt(math::sum($diff.{$value * $value})));
    } ELSE {
        RETURN 0.0;
    };
};

-- Find similar memories function
DEFINE FUNCTION fn::find_similar_memories($memory_id: record<memory_cubes>, $threshold: float, $limit: int) {
    LET $target = SELECT embeddings FROM memory_cubes WHERE id = $memory_id;
    RETURN SELECT *, fn::vector_similarity($target.embeddings, embeddings, "cosine") AS similarity
           FROM memory_cubes 
           WHERE id != $memory_id 
           AND fn::vector_similarity($target.embeddings, embeddings, "cosine") > $threshold
           ORDER BY similarity DESC
           LIMIT $limit;
};

-- Memory quality assessment function
DEFINE FUNCTION fn::assess_memory_quality($memory_id: record<memory_cubes>) {
    LET $memory = SELECT * FROM memory_cubes WHERE id = $memory_id;
    LET $content_score = IF string::len($memory.payload.content) > 10 THEN 0.8 ELSE 0.3;
    LET $metadata_score = IF count($memory.metadata) > 0 THEN 0.7 ELSE 0.3;
    LET $embedding_score = IF array::len($memory.embeddings) > 0 THEN 0.9 ELSE 0.1;
    LET $access_score = math::min(1.0, $memory.access_count / 10.0);
    
    RETURN ($content_score + $metadata_score + $embedding_score + $access_score) / 4.0;
};

-- ===============================================
-- EVENTS FOR REAL-TIME UPDATES
-- ===============================================

-- Memory cube change events
DEFINE EVENT memory_cube_changed ON TABLE memory_cubes WHEN $event = "UPDATE" THEN {
    CREATE memory_operations CONTENT {
        user_id: $after.user_id,
        operation_type: "update",
        memory_ids: [$after.id],
        operation_data: {
            previous_version: $before.version,
            new_version: $after.version,
            changed_fields: array::difference(object::keys($after), object::keys($before))
        },
        status: "completed",
        processing_time_ms: 0,
        completed_at: time::now()
    };
};

-- Quality score update event
DEFINE EVENT quality_update ON TABLE memory_cubes WHEN $event = "UPDATE" AND $before.quality_score != $after.quality_score THEN {
    -- Trigger quality-based subscriptions
    FOR $sub IN (SELECT * FROM memory_subscriptions WHERE subscription_type = "quality_updates" AND is_active = true) {
        -- Implementation would trigger webhook or notification
        UPDATE memory_subscriptions SET last_triggered = time::now() WHERE id = $sub.id;
    }
};

-- ===============================================
-- SCOPE DEFINITIONS FOR AUTHENTICATION
-- ===============================================

-- User scope for authenticated access
DEFINE SCOPE user SESSION 24h
    SIGNUP ( CREATE user SET username = $username, email = $email, password = crypto::argon2::generate($password) )
    SIGNIN ( SELECT * FROM user WHERE username = $username AND crypto::argon2::compare(password, $password) );

-- API scope for programmatic access
DEFINE SCOPE api SESSION 30d
    SIGNIN ( SELECT * FROM api_keys WHERE key_hash = crypto::sha256($api_key) AND is_active = true );

-- ===============================================
-- ANALYZERS FOR FULL-TEXT SEARCH
-- ===============================================

-- Content analyzer for memory search
DEFINE ANALYZER content_analyzer TOKENIZERS blank,class FILTERS lowercase,snowball(english);

-- Metadata analyzer for flexible search
DEFINE ANALYZER metadata_analyzer TOKENIZERS blank,class,camel,punct FILTERS lowercase,ngram(2,3);

-- ===============================================
-- PARAMETERS FOR CONFIGURATION
-- ===============================================

-- System configuration parameters
DEFINE PARAM $max_memory_size DEFAULT 104857600; -- 100MB
DEFINE PARAM $default_similarity_threshold DEFAULT 0.7;
DEFINE PARAM $max_embeddings_dimensions DEFAULT 1536;
DEFINE PARAM $default_ttl_seconds DEFAULT 86400; -- 24 hours

-- ===============================================
-- SAMPLE DATA AND INITIALIZATION
-- ===============================================

-- Create default memory types configuration
CREATE memory_type_config:working CONTENT {
    name: "working",
    description: "Short-term memory for immediate context",
    max_size: 10,
    ttl_seconds: 3600,
    auto_promote: true
};

CREATE memory_type_config:core CONTENT {
    name: "core",
    description: "Essential long-term memories",
    max_size: 100,
    ttl_seconds: null,
    auto_promote: false
};

CREATE memory_type_config:recall CONTENT {
    name: "recall",
    description: "Recently accessed memories",
    max_size: 50,
    ttl_seconds: 86400,
    auto_promote: true
};

CREATE memory_type_config:archival CONTENT {
    name: "archival",
    description: "Long-term storage for infrequently accessed memories",
    max_size: 10000,
    ttl_seconds: null,
    auto_promote: false
};

-- ===============================================
-- LIVE QUERIES FOR REAL-TIME MONITORING
-- ===============================================

-- Monitor memory operations in real-time
-- LIVE SELECT * FROM memory_operations WHERE status = "in_progress";

-- Monitor conflict detection
-- LIVE SELECT * FROM memory_conflicts WHERE status = "detected";

-- Monitor memory quality changes
-- LIVE SELECT id, quality_score FROM memory_cubes WHERE quality_score < 0.5; 