use chrono::NaiveDateTime;
use std::borrow::Cow;
use std::collections::HashMap;

use validator::{Validate, ValidationError, ValidationErrors};

use config::CurrenciesLimits;
use models::*;
use schema::exchanges;

#[derive(Debug, Queryable, Clone)]
pub struct Exchange {
    pub id: ExchangeId,
    pub from_: Currency,
    pub to_: Currency,
    pub amount: Amount,
    pub expiration: NaiveDateTime,
    pub rate: f64,
    pub user_id: UserId,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub amount_currency: Currency,
}

impl Default for Exchange {
    fn default() -> Self {
        Self {
            id: ExchangeId::generate(),
            from_: Currency::Eth,
            to_: Currency::Btc,
            amount: Amount::default(),
            expiration: ::chrono::Utc::now().naive_utc(),
            rate: 0.34343,
            user_id: UserId::generate(),
            created_at: ::chrono::Utc::now().naive_utc(),
            updated_at: ::chrono::Utc::now().naive_utc(),
            amount_currency: Currency::Eth,
        }
    }
}

impl From<NewExchange> for Exchange {
    fn from(new_exchange: NewExchange) -> Self {
        Self {
            id: new_exchange.id,
            from_: new_exchange.from_,
            to_: new_exchange.to_,
            amount: new_exchange.amount,
            rate: new_exchange.rate,
            expiration: new_exchange.expiration,
            user_id: new_exchange.user_id,
            amount_currency: new_exchange.amount_currency,
            ..Default::default()
        }
    }
}

#[derive(Debug, Insertable, Validate, Clone)]
#[table_name = "exchanges"]
pub struct NewExchange {
    pub id: ExchangeId,
    pub from_: Currency,
    pub to_: Currency,
    pub amount: Amount,
    pub expiration: NaiveDateTime,
    pub rate: f64,
    pub user_id: UserId,
    pub amount_currency: Currency,
}

impl Default for NewExchange {
    fn default() -> Self {
        Self {
            id: ExchangeId::generate(),
            from_: Currency::Eth,
            to_: Currency::Btc,
            amount: Amount::default(),
            expiration: ::chrono::Utc::now().naive_utc(),
            rate: 0.34343,
            user_id: UserId::generate(),
            amount_currency: Currency::Eth,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateSellOrder {
    pub id: ExchangeId,
    pub from: Currency,
    pub to: Currency,
    pub actual_amount: Amount,
    pub amount_currency: Currency,
}

impl From<CreateSellOrder> for GetExchange {
    fn from(sell: CreateSellOrder) -> Self {
        Self {
            id: sell.id,
            from: sell.from,
            to: sell.to,
            actual_amount: sell.actual_amount,
            amount_currency: sell.amount_currency,
        }
    }
}

impl Default for CreateSellOrder {
    fn default() -> Self {
        Self {
            id: ExchangeId::generate(),
            from: Currency::Eth,
            to: Currency::Btc,
            actual_amount: Amount::default(),
            amount_currency: Currency::Eth,
        }
    }
}

pub fn validate(amount_currency: Currency, amount: Amount, limits: CurrenciesLimits) -> Result<(), ValidationErrors> {
    let limit = match amount_currency {
        Currency::Btc => limits.btc,
        Currency::Eth => limits.eth,
        Currency::Stq => limits.stq,
        Currency::Usd => limits.usd,
        Currency::Rub => limits.rub,
    };
    let quantity = amount_currency.to_f64(amount);

    let mut errors = ValidationErrors::new();
    if quantity < limit.min || quantity > limit.max {
        let error = ValidationError {
            code: Cow::from("limit"),
            message: Some(Cow::from(format!(
                "Amount should be between {} and {}",
                amount_currency.from_f64(limit.min),
                amount_currency.from_f64(limit.max)
            ))),
            params: HashMap::new(),
        };
        errors.add("actual_amount", error);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Debug, Clone)]
pub struct GetExchange {
    pub id: ExchangeId,
    pub from: Currency,
    pub to: Currency,
    pub actual_amount: Amount,
    pub amount_currency: Currency,
}

impl Default for GetExchange {
    fn default() -> Self {
        Self {
            id: ExchangeId::generate(),
            from: Currency::Eth,
            to: Currency::Btc,
            actual_amount: Amount::default(),
            amount_currency: Currency::Eth,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetRate {
    pub id: ExchangeId,
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub amount_currency: Currency,
}

impl NewExchange {
    pub fn new(req: GetRate, expiration: NaiveDateTime, rate: f64, user_id: UserId) -> Self {
        Self {
            id: ExchangeId::generate(),
            from_: req.from,
            to_: req.to,
            amount: req.amount,
            amount_currency: req.amount_currency,
            expiration,
            rate,
            user_id,
        }
    }
}

impl Default for GetRate {
    fn default() -> Self {
        Self {
            id: ExchangeId::generate(),
            from: Currency::Eth,
            to: Currency::Btc,
            amount: Amount::default(),
            amount_currency: Currency::Eth,
        }
    }
}
