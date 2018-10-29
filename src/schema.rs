table! {
    exchanges (id) {
        id -> Uuid,
        from_ -> Varchar,
        to_ -> Varchar,
        amount -> Numeric,
        expiration -> Timestamp,
        rate -> Float8,
        user_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Uuid,
        name -> Varchar,
        authentication_token -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
