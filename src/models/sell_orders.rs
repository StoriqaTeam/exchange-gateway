use chrono::NaiveDateTime;

use serde_json::Value;

use models::*;
use schema::sell_orders;

#[derive(Debug, Queryable, Clone)]
pub struct SellOrderDB {
    pub id: i32,
    pub data: Option<Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Default for SellOrderDB {
    fn default() -> Self {
        Self {
            id: 0,
            data: None,
            created_at: ::chrono::Utc::now().naive_utc(),
            updated_at: ::chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<NewSellOrder> for SellOrderDB {
    fn from(new_exchange: NewSellOrder) -> Self {
        Self {
            data: new_exchange.data,
            ..Default::default()
        }
    }
}

#[derive(Debug, Queryable, Clone)]
pub struct SellOrder {
    pub from: Currency,
    pub to: Currency,
    pub actual_amount: Amount,
}

impl SellOrder {
    pub fn new(actual_quantity: f64, from: Currency, to: Currency) -> Self {
        let actual_amount = to.from_f64(actual_quantity);
        Self { actual_amount, from, to }
    }
}

#[derive(Debug, Insertable, Clone, Default)]
#[table_name = "sell_orders"]
pub struct NewSellOrder {
    pub data: Option<Value>,
}

impl NewSellOrder {
    pub fn new(data: Option<Value>) -> Self {
        Self { data }
    }
}
