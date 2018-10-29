-- Your SQL goes here
CREATE TABLE exchanges (
    id UUID PRIMARY KEY,
    from_ VARCHAR NOT NULL,
    to_ VARCHAR NOT NULL,
    amount NUMERIC NOT NULL,
    expiration TIMESTAMP NOT NULL,
    rate DOUBLE PRECISION NOT NULL,
    user_id UUID NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
    updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);

SELECT diesel_manage_updated_at('exchanges');
