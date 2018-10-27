use std::time::SystemTime;

use validator::Validate;

use models::*;
use schema::exchanges;

#[derive(Debug, Queryable, Clone)]
pub struct Exchange {
    pub id: ExchangeId,
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub reserved_for: i32,
    pub rate: f64,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl Default for Exchange {
    fn default() -> Self {
        Self {
            id: ExchangeId::generate(),
            from_currency: Currency::Eth,
            to_currency: Currency::Btc,
            amount: Amount::default(),
            reserved_for: 600,
            rate: 0.34343,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        }
    }
}

impl From<NewExchange> for Exchange {
    fn from(new_exchange: NewExchange) -> Self {
        Self {
            id: new_exchange.id,
            from_currency: new_exchange.from_currency,
            to_currency: new_exchange.to_currency,
            amount: new_exchange.amount,
            rate: new_exchange.rate,
            reserved_for: new_exchange.reserved_for,
            ..Default::default()
        }
    }
}

#[derive(Debug, Insertable, Validate, Clone)]
#[table_name = "exchanges"]
pub struct NewExchange {
    pub id: ExchangeId,
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub reserved_for: i32,
    pub rate: f64,
}

impl Default for NewExchange {
    fn default() -> Self {
        Self {
            id: ExchangeId::generate(),
            from_currency: Currency::Eth,
            to_currency: Currency::Btc,
            amount: Amount::default(),
            reserved_for: 600,
            rate: 0.34343,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateSellOrder {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
    pub to: AccountAddress,
    pub from: AccountAddress,
}

impl From<CreateSellOrder> for GetExchange {
    fn from(sell: CreateSellOrder) -> Self {
        Self {
            from_currency: sell.from_currency,
            to_currency: sell.to_currency,
            amount: sell.amount,
            rate: sell.rate,
        }
    }
}

impl Default for CreateSellOrder {
    fn default() -> Self {
        Self {
            from_currency: Currency::Eth,
            to_currency: Currency::Btc,
            amount: Amount::default(),
            rate: 0.333,
            to: AccountAddress::default(),
            from: AccountAddress::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetExchange {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl Default for GetExchange {
    fn default() -> Self {
        Self {
            from_currency: Currency::Eth,
            to_currency: Currency::Btc,
            amount: Amount::default(),
            rate: 0.34343,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SellOrder {
    pub id: BlockchainTransactionId,
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
    pub to: AccountAddress,
    pub from: AccountAddress,
}

#[derive(Debug, Insertable, Validate, Clone)]
#[table_name = "exchanges"]
pub struct ExchangeRequest {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
}

impl NewExchange {
    pub fn new(req: ExchangeRequest, reserved_for: i32, rate: f64) -> Self {
        Self {
            id: ExchangeId::generate(),
            from_currency: req.from_currency,
            to_currency: req.to_currency,
            amount: req.amount,
            reserved_for,
            rate,
        }
    }
}

impl Default for ExchangeRequest {
    fn default() -> Self {
        Self {
            from_currency: Currency::Eth,
            to_currency: Currency::Btc,
            amount: Amount::default(),
        }
    }
}
