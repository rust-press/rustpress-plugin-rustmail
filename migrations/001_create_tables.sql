-- RustMail Database Schema
-- Migration: 001_create_tables

-- Email templates table
CREATE TABLE IF NOT EXISTS email_templates (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    slug VARCHAR(100) NOT NULL UNIQUE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    template_type VARCHAR(50) NOT NULL DEFAULT 'transactional',
    subject TEXT NOT NULL,
    text_body TEXT,
    html_body TEXT,
    preheader VARCHAR(500),
    layout_id UUID REFERENCES email_layouts(id) ON DELETE SET NULL,
    variables JSONB NOT NULL DEFAULT '[]',
    default_from VARCHAR(255),
    default_reply_to VARCHAR(255),
    tags TEXT[] DEFAULT '{}',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    version INTEGER NOT NULL DEFAULT 1,
    created_by UUID,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for templates
CREATE INDEX IF NOT EXISTS idx_email_templates_slug ON email_templates(slug);
CREATE INDEX IF NOT EXISTS idx_email_templates_type ON email_templates(template_type);
CREATE INDEX IF NOT EXISTS idx_email_templates_active ON email_templates(active);

-- Email layouts table
CREATE TABLE IF NOT EXISTS email_layouts (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    slug VARCHAR(100) NOT NULL UNIQUE,
    html TEXT NOT NULL,
    text TEXT,
    description TEXT,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index for layouts
CREATE INDEX IF NOT EXISTS idx_email_layouts_slug ON email_layouts(slug);

-- Email queue table
CREATE TABLE IF NOT EXISTS email_queue (
    id UUID PRIMARY KEY,
    email_data JSONB NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    last_error TEXT,
    scheduled_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    next_retry_at TIMESTAMP WITH TIME ZONE,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    priority INTEGER NOT NULL DEFAULT 0,
    worker_id VARCHAR(100)
);

-- Create indexes for queue
CREATE INDEX IF NOT EXISTS idx_email_queue_status ON email_queue(status);
CREATE INDEX IF NOT EXISTS idx_email_queue_scheduled ON email_queue(scheduled_at);
CREATE INDEX IF NOT EXISTS idx_email_queue_priority ON email_queue(priority DESC);
CREATE INDEX IF NOT EXISTS idx_email_queue_next_retry ON email_queue(next_retry_at);
CREATE INDEX IF NOT EXISTS idx_email_queue_worker ON email_queue(worker_id);

-- Partial index for pending items
CREATE INDEX IF NOT EXISTS idx_email_queue_pending ON email_queue(scheduled_at, priority DESC)
    WHERE status IN ('pending', 'deferred');

-- Email logs table
CREATE TABLE IF NOT EXISTS email_logs (
    id UUID PRIMARY KEY,
    email_id UUID NOT NULL,
    queue_id UUID REFERENCES email_queue(id) ON DELETE SET NULL,
    event VARCHAR(50) NOT NULL,
    recipient VARCHAR(255) NOT NULL,
    subject VARCHAR(500),
    template_id UUID REFERENCES email_templates(id) ON DELETE SET NULL,
    template_name VARCHAR(100),
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    provider_message_id VARCHAR(255),
    provider VARCHAR(50) NOT NULL DEFAULT 'smtp',
    provider_response TEXT,
    error TEXT,
    ip_address INET,
    user_agent TEXT,
    click_url TEXT,
    metadata JSONB DEFAULT '{}'
);

-- Create indexes for logs
CREATE INDEX IF NOT EXISTS idx_email_logs_email ON email_logs(email_id);
CREATE INDEX IF NOT EXISTS idx_email_logs_recipient ON email_logs(recipient);
CREATE INDEX IF NOT EXISTS idx_email_logs_event ON email_logs(event);
CREATE INDEX IF NOT EXISTS idx_email_logs_timestamp ON email_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_email_logs_template ON email_logs(template_id);
CREATE INDEX IF NOT EXISTS idx_email_logs_provider ON email_logs(provider);

-- Partial indexes for common queries
CREATE INDEX IF NOT EXISTS idx_email_logs_errors ON email_logs(timestamp DESC)
    WHERE error IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_email_logs_opens ON email_logs(timestamp DESC)
    WHERE event = 'opened';

CREATE INDEX IF NOT EXISTS idx_email_logs_clicks ON email_logs(timestamp DESC)
    WHERE event = 'clicked';

-- Bounce records table
CREATE TABLE IF NOT EXISTS email_bounces (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    bounce_type VARCHAR(20) NOT NULL,
    reason TEXT,
    diagnostic TEXT,
    first_bounce TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_bounce TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    bounce_count INTEGER NOT NULL DEFAULT 1,
    suppressed BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(email)
);

-- Create indexes for bounces
CREATE INDEX IF NOT EXISTS idx_email_bounces_email ON email_bounces(email);
CREATE INDEX IF NOT EXISTS idx_email_bounces_type ON email_bounces(bounce_type);
CREATE INDEX IF NOT EXISTS idx_email_bounces_suppressed ON email_bounces(suppressed);

-- Complaint records table
CREATE TABLE IF NOT EXISTS email_complaints (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    complaint_type VARCHAR(50) NOT NULL,
    email_id UUID,
    feedback_id VARCHAR(255),
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    user_agent TEXT,
    suppressed BOOLEAN NOT NULL DEFAULT TRUE,
    UNIQUE(email)
);

-- Create indexes for complaints
CREATE INDEX IF NOT EXISTS idx_email_complaints_email ON email_complaints(email);
CREATE INDEX IF NOT EXISTS idx_email_complaints_type ON email_complaints(complaint_type);

-- Suppression list table
CREATE TABLE IF NOT EXISTS email_suppression (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    reason VARCHAR(50) NOT NULL,
    source_id UUID,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE
);

-- Create indexes for suppression
CREATE INDEX IF NOT EXISTS idx_email_suppression_email ON email_suppression(email);
CREATE INDEX IF NOT EXISTS idx_email_suppression_reason ON email_suppression(reason);

-- Unsubscribe records table
CREATE TABLE IF NOT EXISTS email_unsubscribes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    list VARCHAR(100),
    email_id UUID,
    reason TEXT,
    ip_address INET,
    user_agent TEXT,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(email, list)
);

-- Create indexes for unsubscribes
CREATE INDEX IF NOT EXISTS idx_email_unsubscribes_email ON email_unsubscribes(email);
CREATE INDEX IF NOT EXISTS idx_email_unsubscribes_list ON email_unsubscribes(list);

-- Email settings table
CREATE TABLE IF NOT EXISTS email_settings (
    key VARCHAR(100) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Insert default settings
INSERT INTO email_settings (key, value) VALUES
    ('smtp_host', '"localhost"'),
    ('smtp_port', '25'),
    ('smtp_tls', '"starttls"'),
    ('default_from_email', '""'),
    ('default_from_name', '""'),
    ('track_opens', 'true'),
    ('track_clicks', 'true'),
    ('queue_enabled', 'true'),
    ('retry_max_attempts', '3'),
    ('retry_initial_delay', '60')
ON CONFLICT (key) DO NOTHING;

-- Email statistics view
CREATE OR REPLACE VIEW email_stats AS
SELECT
    DATE_TRUNC('day', timestamp) as date,
    event,
    COUNT(*) as count
FROM email_logs
WHERE timestamp > NOW() - INTERVAL '30 days'
GROUP BY DATE_TRUNC('day', timestamp), event
ORDER BY date DESC, event;

-- Daily email summary view
CREATE OR REPLACE VIEW email_daily_summary AS
SELECT
    DATE_TRUNC('day', timestamp) as date,
    COUNT(*) FILTER (WHERE event = 'sent') as sent,
    COUNT(*) FILTER (WHERE event = 'delivered') as delivered,
    COUNT(*) FILTER (WHERE event IN ('bounced', 'hard_bounce', 'soft_bounce')) as bounced,
    COUNT(*) FILTER (WHERE event = 'opened') as opened,
    COUNT(*) FILTER (WHERE event = 'clicked') as clicked,
    COUNT(*) FILTER (WHERE event = 'spam_complaint') as spam,
    COUNT(*) FILTER (WHERE event = 'failed') as failed
FROM email_logs
WHERE timestamp > NOW() - INTERVAL '30 days'
GROUP BY DATE_TRUNC('day', timestamp)
ORDER BY date DESC;

-- Functions

-- Function to update queue item status
CREATE OR REPLACE FUNCTION claim_queue_item(
    p_limit INTEGER,
    p_worker_id VARCHAR(100)
)
RETURNS SETOF email_queue AS $$
BEGIN
    RETURN QUERY
    UPDATE email_queue
    SET status = 'processing',
        started_at = NOW(),
        worker_id = p_worker_id,
        attempts = attempts + 1
    WHERE id IN (
        SELECT id FROM email_queue
        WHERE status IN ('pending', 'deferred')
        AND scheduled_at <= NOW()
        AND (next_retry_at IS NULL OR next_retry_at <= NOW())
        ORDER BY priority DESC, scheduled_at ASC
        LIMIT p_limit
        FOR UPDATE SKIP LOCKED
    )
    RETURNING *;
END;
$$ LANGUAGE plpgsql;

-- Function to calculate retry delay
CREATE OR REPLACE FUNCTION calculate_retry_delay(attempts INTEGER)
RETURNS INTERVAL AS $$
BEGIN
    -- Exponential backoff: 1min, 2min, 4min, etc.
    RETURN (POWER(2, attempts) * INTERVAL '1 minute');
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to clean up old queue items
CREATE OR REPLACE FUNCTION cleanup_old_queue_items(days INTEGER DEFAULT 30)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM email_queue
    WHERE status IN ('sent', 'failed', 'cancelled')
    AND completed_at < NOW() - (days || ' days')::INTERVAL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up old logs
CREATE OR REPLACE FUNCTION cleanup_old_logs(days INTEGER DEFAULT 90)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM email_logs
    WHERE timestamp < NOW() - (days || ' days')::INTERVAL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-suppress on hard bounce
CREATE OR REPLACE FUNCTION auto_suppress_on_bounce()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.bounce_type = 'hard' OR NEW.bounce_count >= 3 THEN
        INSERT INTO email_suppression (email, reason, source_id)
        VALUES (NEW.email, 'bounce', NEW.id)
        ON CONFLICT (email) DO NOTHING;

        NEW.suppressed := TRUE;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS email_bounces_auto_suppress ON email_bounces;
CREATE TRIGGER email_bounces_auto_suppress
    BEFORE INSERT OR UPDATE ON email_bounces
    FOR EACH ROW EXECUTE FUNCTION auto_suppress_on_bounce();

-- Trigger to auto-suppress on complaint
CREATE OR REPLACE FUNCTION auto_suppress_on_complaint()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.complaint_type = 'abuse' THEN
        INSERT INTO email_suppression (email, reason, source_id)
        VALUES (NEW.email, 'spam_complaint', NEW.id)
        ON CONFLICT (email) DO NOTHING;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS email_complaints_auto_suppress ON email_complaints;
CREATE TRIGGER email_complaints_auto_suppress
    AFTER INSERT ON email_complaints
    FOR EACH ROW EXECUTE FUNCTION auto_suppress_on_complaint();

-- Permissions (adjust for your auth system)
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO rustpress_app;
-- GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO rustpress_app;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO rustpress_app;
