CREATE TABLE IF NOT EXISTS inventory (
    id UUID PRIMARY KEY NOT NULL UNIQUE,
    variant_id UUID NOT NULL UNIQUE,
    stock_quantity INT NOT NULL DEFAULT 0 CHECK (stock_quantity >= 0),
    reserved_quantity INT NOT NULL DEFAULT 0 CHECK (reserved_quantity >= 0),
    low_stock_threshold INT NOT NULL DEFAULT 5 CHECK (low_stock_threshold >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (reserved_quantity <= stock_quantity)
);
CREATE TABLE IF NOT EXISTS reservations (
    id UUID PRIMARY KEY,
    reservation_key TEXT NOT NULL UNIQUE,
    variant_id UUID NOT NULL REFERENCES inventory(variant_id),
    quantity INT NOT NULL CHECK (quantity > 0),
    status TEXT NOT NULL CHECK (status IN ('pending', 'committed', 'released')),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_reservations_expiry ON reservations(status, expires_at)
WHERE status = 'pending';
CREATE TABLE IF NOT EXISTS inventory_logs (
    id UUID PRIMARY KEY,
    sku VARCHAR(100) NOT NULL,
    change_amount INT NOT NULL,
    type VARCHAR(50) NOT NULL,
    reference_id VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS outbox_events (
    id UUID PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    published_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_outbox_unpublished ON outbox_events(created_at)
WHERE published_at IS NULL;