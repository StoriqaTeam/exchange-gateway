use std::time::SystemTime;

use serde_json::Value;

use models::*;
use schema::sell_orders;

#[derive(Debug, Queryable, Clone)]
pub struct SellOrderDB {
    pub id: Nonce,
    pub data: Option<Value>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl Default for SellOrderDB {
    fn default() -> Self {
        Self {
            id: Nonce::generate(),
            data: None,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
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
