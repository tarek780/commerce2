CREATE TABLE payments (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    amount NUMERIC(14, 2) NOT NULL,
    currency CHAR(3) NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN (
            'pending',
            'processing',
            'succeeded',
            'failed',
            'cancelled',
            'expired',
            'refunded'
        )
    ),
    gateway TEXT NOT NULL,
    gateway_reference TEXT,
    payment_url TEXT,
    failure_reason TEXT,
    customer_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    reservation_expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_payments_order ON payments(order_id);
CREATE INDEX idx_payments_status ON payments(status);
CREATE TABLE payment_attempts (
    id BIGSERIAL PRIMARY KEY,
    payment_id TEXT NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    request JSONB NOT NULL,
    response JSONB,
    success BOOLEAN NOT NULL DEFAULT FALSE,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE processed_webhooks (
    gateway_event_id TEXT PRIMARY KEY,
    gateway TEXT NOT NULL,
    payment_id TEXT,
    event_type TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE outbox_events (
    id TEXT PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ
);
CREATE INDEX idx_outbox_unpublished ON outbox_events(created_at)
WHERE published_at IS NULL;