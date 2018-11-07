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
        amount_currency -> Varchar,
    }
}

table! {
    sell_orders (id) {
        id -> Int4,
        data -> Nullable<Jsonb>,
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

allow_tables_to_appear_in_same_query!(exchanges, sell_orders, users,);
