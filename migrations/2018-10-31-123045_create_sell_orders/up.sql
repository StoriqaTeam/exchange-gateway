-- Your SQL goes here
CREATE TABLE sell_orders (
    id SERIAL PRIMARY KEY,
    data JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
    updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('sell_orders');
