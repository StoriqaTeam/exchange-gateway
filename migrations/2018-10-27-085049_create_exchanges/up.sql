-- Your SQL goes here
CREATE TABLE exchanges (
    id UUID PRIMARY KEY,
    from_currency VARCHAR NOT NULL,
    to_currency VARCHAR NOT NULL,
    amount NUMERIC NOT NULL,
    reserved_for INTEGER NOT NULL,
    rate DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
    updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('exchanges');
