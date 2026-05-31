-- ===============================================
-- GaussOS PostgreSQL Schema v2.0
-- Focus: User Management, Security, System Config, Audit Logs
-- ===============================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "btree_gin";
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ===============================================
-- USERS & AUTHENTICATION
-- ===============================================

-- Core users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    salt VARCHAR(255) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    is_admin BOOLEAN DEFAULT FALSE,
    is_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login TIMESTAMPTZ,
    failed_login_attempts INTEGER DEFAULT 0,
    locked_until TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}',
    CONSTRAINT users_username_length CHECK (char_length(username) >= 3),
    CONSTRAINT users_email_format CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$')
);

-- User sessions with enhanced security
CREATE TABLE user_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    refresh_token_hash VARCHAR(255),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_accessed TIMESTAMPTZ DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT,
    device_fingerprint VARCHAR(255),
    is_revoked BOOLEAN DEFAULT FALSE,
    revoked_at TIMESTAMPTZ,
    revoked_by UUID REFERENCES users(id),
    session_data JSONB DEFAULT '{}'
);

-- API keys for programmatic access
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    permissions JSONB DEFAULT '[]',
    rate_limit_per_hour INTEGER DEFAULT 1000,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used TIMESTAMPTZ,
    usage_count BIGINT DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE
);

-- ===============================================
-- SECURITY & PERMISSIONS
-- ===============================================

-- Roles and permissions
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    permissions JSONB DEFAULT '[]',
    is_system_role BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- User role assignments
CREATE TABLE user_roles (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ DEFAULT NOW(),
    assigned_by UUID REFERENCES users(id),
    expires_at TIMESTAMPTZ,
    PRIMARY KEY (user_id, role_id)
);

-- Memory namespace permissions
CREATE TABLE memory_namespace_permissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    namespace VARCHAR(255) NOT NULL,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID REFERENCES roles(id) ON DELETE CASCADE,
    permission_type VARCHAR(20) NOT NULL CHECK (permission_type IN ('read', 'write', 'delete', 'admin', 'execute')),
    granted_at TIMESTAMPTZ DEFAULT NOW(),
    granted_by UUID REFERENCES users(id),
    expires_at TIMESTAMPTZ,
    conditions JSONB DEFAULT '{}',
    CHECK (user_id IS NOT NULL OR role_id IS NOT NULL)
);

-- ===============================================
-- SYSTEM CONFIGURATION
-- ===============================================

-- Enhanced system configuration
CREATE TABLE system_config (
    key VARCHAR(255) PRIMARY KEY,
    value JSONB NOT NULL,
    value_type VARCHAR(50) DEFAULT 'string',
    description TEXT,
    is_secret BOOLEAN DEFAULT FALSE,
    validation_schema JSONB,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    updated_by UUID REFERENCES users(id),
    version INTEGER DEFAULT 1,
    tags VARCHAR(255)[]
);

-- Feature flags
CREATE TABLE feature_flags (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) UNIQUE NOT NULL,
    description TEXT,
    is_enabled BOOLEAN DEFAULT FALSE,
    conditions JSONB DEFAULT '{}',
    rollout_percentage INTEGER DEFAULT 0 CHECK (rollout_percentage >= 0 AND rollout_percentage <= 100),
    target_users UUID[],
    target_roles UUID[],
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID REFERENCES users(id)
);

-- ===============================================
-- AUDIT & MONITORING
-- ===============================================

-- Comprehensive audit logs
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id),
    session_id UUID REFERENCES user_sessions(id),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),
    namespace VARCHAR(255),
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(255),
    duration_ms INTEGER,
    status_code INTEGER,
    error_message TEXT,
    metadata JSONB DEFAULT '{}',
    INDEX idx_audit_logs_user_timestamp (user_id, timestamp),
    INDEX idx_audit_logs_action_timestamp (action, timestamp),
    INDEX idx_audit_logs_resource (resource_type, resource_id)
);

-- Security events
CREATE TABLE security_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    event_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) DEFAULT 'medium' CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    user_id UUID REFERENCES users(id),
    ip_address INET,
    user_agent TEXT,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    details JSONB DEFAULT '{}',
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users(id),
    INDEX idx_security_events_type_timestamp (event_type, timestamp),
    INDEX idx_security_events_severity_timestamp (severity, timestamp)
);

-- Performance metrics
CREATE TABLE performance_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_name VARCHAR(255) NOT NULL,
    metric_value NUMERIC NOT NULL,
    metric_type VARCHAR(50) DEFAULT 'gauge',
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    labels JSONB DEFAULT '{}',
    namespace VARCHAR(255),
    INDEX idx_performance_metrics_name_timestamp (metric_name, timestamp)
);

-- ===============================================
-- MEMORY METADATA & RELATIONSHIPS
-- ===============================================

-- Memory reference table (actual data in SurrealDB)
CREATE TABLE memory_references (
    id UUID PRIMARY KEY,
    surreal_id VARCHAR(255) NOT NULL UNIQUE,
    namespace VARCHAR(255) NOT NULL,
    memory_type VARCHAR(100),
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    size_bytes BIGINT DEFAULT 0,
    access_count BIGINT DEFAULT 0,
    last_accessed TIMESTAMPTZ,
    tags VARCHAR(255)[],
    is_archived BOOLEAN DEFAULT FALSE,
    INDEX idx_memory_refs_namespace (namespace),
    INDEX idx_memory_refs_type (memory_type),
    INDEX idx_memory_refs_created_by (created_by),
    INDEX idx_memory_refs_tags USING gin(tags)
);

-- ===============================================
-- SCHEDULED TASKS & JOBS
-- ===============================================

-- Background jobs
CREATE TABLE background_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_type VARCHAR(100) NOT NULL,
    status VARCHAR(50) DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    priority INTEGER DEFAULT 5 CHECK (priority >= 1 AND priority <= 10),
    payload JSONB DEFAULT '{}',
    result JSONB,
    error_message TEXT,
    scheduled_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_by UUID REFERENCES users(id),
    max_retries INTEGER DEFAULT 3,
    retry_count INTEGER DEFAULT 0,
    next_retry_at TIMESTAMPTZ,
    INDEX idx_background_jobs_status_priority (status, priority),
    INDEX idx_background_jobs_scheduled_at (scheduled_at)
);

-- ===============================================
-- INDEXES FOR PERFORMANCE
-- ===============================================

-- User and authentication indexes
CREATE INDEX idx_users_email_active ON users(email, is_active) WHERE is_active = true;
CREATE INDEX idx_users_username_active ON users(username, is_active) WHERE is_active = true;
CREATE INDEX idx_user_sessions_expires ON user_sessions(expires_at) WHERE is_revoked = false;
CREATE INDEX idx_user_sessions_token ON user_sessions(token_hash) WHERE is_revoked = false;
CREATE INDEX idx_api_keys_active ON api_keys(key_hash) WHERE is_active = true;

-- Permission indexes
CREATE INDEX idx_user_roles_user ON user_roles(user_id);
CREATE INDEX idx_memory_namespace_perms_namespace ON memory_namespace_permissions(namespace);
CREATE INDEX idx_memory_namespace_perms_user ON memory_namespace_permissions(user_id);

-- System indexes
CREATE INDEX idx_system_config_tags ON system_config USING gin(tags);
CREATE INDEX idx_feature_flags_enabled ON feature_flags(name) WHERE is_enabled = true;

-- ===============================================
-- FUNCTIONS AND TRIGGERS
-- ===============================================

-- Update timestamp trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply triggers
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_roles_updated_at BEFORE UPDATE ON roles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_feature_flags_updated_at BEFORE UPDATE ON feature_flags
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_memory_references_updated_at BEFORE UPDATE ON memory_references
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Security functions
CREATE OR REPLACE FUNCTION check_user_permissions(
    p_user_id UUID,
    p_namespace VARCHAR(255),
    p_permission VARCHAR(20)
)
RETURNS BOOLEAN AS $$
DECLARE
    has_permission BOOLEAN := FALSE;
BEGIN
    -- Check direct user permissions
    SELECT EXISTS(
        SELECT 1 FROM memory_namespace_permissions mnp
        WHERE mnp.user_id = p_user_id
        AND mnp.namespace = p_namespace
        AND mnp.permission_type = p_permission
        AND (mnp.expires_at IS NULL OR mnp.expires_at > NOW())
    ) INTO has_permission;
    
    IF has_permission THEN
        RETURN TRUE;
    END IF;
    
    -- Check role-based permissions
    SELECT EXISTS(
        SELECT 1 FROM memory_namespace_permissions mnp
        JOIN user_roles ur ON mnp.role_id = ur.role_id
        WHERE ur.user_id = p_user_id
        AND mnp.namespace = p_namespace
        AND mnp.permission_type = p_permission
        AND (mnp.expires_at IS NULL OR mnp.expires_at > NOW())
        AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
    ) INTO has_permission;
    
    RETURN has_permission;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Cleanup functions
CREATE OR REPLACE FUNCTION cleanup_expired_sessions()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER := 0;
BEGIN
    DELETE FROM user_sessions 
    WHERE expires_at < NOW() OR is_revoked = true;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- ===============================================
-- DEFAULT DATA
-- ===============================================

-- Default roles
INSERT INTO roles (name, description, permissions, is_system_role) VALUES
('admin', 'System Administrator', '["*"]', true),
('user', 'Regular User', '["memory:read", "memory:write"]', true),
('readonly', 'Read-only User', '["memory:read"]', true),
('api', 'API Access', '["api:read", "api:write"]', true);

-- Default system configuration
INSERT INTO system_config (key, value, value_type, description, tags) VALUES
('system.version', '"2.0.0"', 'string', 'GaussOS system version', ARRAY['system']),
('memory.default_ttl', '86400', 'integer', 'Default TTL for memories in seconds', ARRAY['memory']),
('memory.max_payload_size', '104857600', 'integer', 'Maximum payload size in bytes (100MB)', ARRAY['memory']),
('cache.max_size', '100000', 'integer', 'Maximum number of memories in cache', ARRAY['cache']),
('security.session_timeout', '3600', 'integer', 'Session timeout in seconds', ARRAY['security']),
('security.max_login_attempts', '5', 'integer', 'Maximum failed login attempts before lockout', ARRAY['security']),
('security.lockout_duration', '900', 'integer', 'Account lockout duration in seconds', ARRAY['security']),
('performance.batch_size', '1000', 'integer', 'Default batch size for operations', ARRAY['performance']),
('api.rate_limit_default', '1000', 'integer', 'Default API rate limit per hour', ARRAY['api']),
('features.simd_enabled', 'true', 'boolean', 'Enable SIMD acceleration', ARRAY['features']);

-- Default feature flags
INSERT INTO feature_flags (name, description, is_enabled, rollout_percentage) VALUES
('advanced_memory_operations', 'Enable advanced memory operations (merge, split, evolve)', true, 100),
('ml_quality_validation', 'Enable ML-powered quality validation', true, 50),
('graph_memory_relationships', 'Enable graph-based memory relationships', true, 100),
('real_time_updates', 'Enable real-time memory updates', false, 0);

-- Create views for common queries
CREATE VIEW active_users AS
SELECT id, username, email, created_at, last_login, is_admin
FROM users 
WHERE is_active = true;

CREATE VIEW user_permissions_view AS
SELECT 
    u.id as user_id,
    u.username,
    mnp.namespace,
    mnp.permission_type,
    CASE 
        WHEN mnp.user_id IS NOT NULL THEN 'direct'
        ELSE 'role'
    END as permission_source,
    COALESCE(r.name, 'direct') as role_name
FROM users u
LEFT JOIN user_roles ur ON u.id = ur.user_id AND (ur.expires_at IS NULL OR ur.expires_at > NOW())
LEFT JOIN roles r ON ur.role_id = r.id
LEFT JOIN memory_namespace_permissions mnp ON (mnp.user_id = u.id OR mnp.role_id = r.id)
WHERE u.is_active = true
AND (mnp.expires_at IS NULL OR mnp.expires_at > NOW());

-- ===============================================
-- PERFORMANCE OPTIMIZATION
-- ===============================================

-- Partitioning for large tables (audit_logs, performance_metrics)
-- Note: Implement based on actual usage patterns

-- Example for audit_logs partitioning by month
-- CREATE TABLE audit_logs_y2024m01 PARTITION OF audit_logs
--     FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- ===============================================
-- SECURITY POLICIES (RLS)
-- ===============================================

-- Enable RLS on sensitive tables
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE memory_references ENABLE ROW LEVEL SECURITY;

-- Policies for users table
CREATE POLICY users_own_data ON users
    FOR ALL
    TO authenticated_users
    USING (id = current_setting('app.current_user_id')::uuid);

-- Admin can see all users
CREATE POLICY users_admin_access ON users
    FOR ALL
    TO admin_users
    USING (true);

-- Grant permissions (adjust as needed)
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO gaussos_app;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO gaussos_app;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO gaussos_app; 