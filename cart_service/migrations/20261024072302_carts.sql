CREATE TABLE carts (
    id UUID PRIMARY KEY UNIQUE NOT NULL,
    user_id UUID UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE cart_items (
    id UUID PRIMARY KEY UNIQUE NOT NULL,
    cart_id UUID NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    variant_id UUID NOT NULL,
    quantity INT NOT NULL CHECK (quantity > 0),
    price_at_add NUMERIC(12, 2) NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (cart_id, variant_id)
);
CREATE INDEX idx_cart_items_cart_id ON cart_items(cart_id);