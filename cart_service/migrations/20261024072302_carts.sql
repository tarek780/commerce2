CREATE TABLE carts (
    id TEXT PRIMARY KEY,
    user_id TEXT UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE cart_items (
    id TEXT PRIMARY KEY,
    cart_id TEXT NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    sku_id TEXT NOT NULL,
    quantity INT NOT NULL CHECK (quantity > 0),
    price_at_add NUMERIC(12, 2) NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (cart_id, sku_id)
);
CREATE INDEX idx_cart_items_cart_id ON cart_items(cart_id);