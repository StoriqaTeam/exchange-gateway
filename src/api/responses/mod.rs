use std::collections::HashMap;

use chrono::NaiveDateTime;

use models::*;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsersResponse {
    pub id: UserId,
    pub name: String,
    pub authentication_token: AuthenticationToken,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<User> for UsersResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            authentication_token: user.authentication_token,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SellOrderResponse {
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
}

impl From<SellOrder> for SellOrderResponse {
    fn from(sell: SellOrder) -> Self {
        Self {
            from: sell.from,
            to: sell.to,
            amount: sell.actual_amount,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeResponse {
    pub id: ExchangeId,
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub expiration: NaiveDateTime,
    pub rate: f64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub amount_currency: Currency,
}

impl From<Exchange> for ExchangeResponse {
    fn from(ex: Exchange) -> Self {
        Self {
            id: ex.id,
            from: ex.from_,
            to: ex.to_,
            amount: ex.amount,
            expiration: ex.expiration,
            rate: ex.rate,
            created_at: ex.created_at,
            updated_at: ex.updated_at,
            amount_currency: ex.amount_currency,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRefreshResponse {
    pub exchange: ExchangeResponse,
    pub is_new_rate: bool,
}

impl From<ExchangeRefresh> for ExchangeRefreshResponse {
    fn from(er: ExchangeRefresh) -> Self {
        let ExchangeRefresh { exchange, is_new_rate } = er;

        Self {
            exchange: ExchangeResponse::from(exchange),
            is_new_rate,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetricsResponse {
    pub balances: HashMap<Currency, f64>,
}

impl From<Metrics> for MetricsResponse {
    fn from(m: Metrics) -> Self {
        Self { balances: m.balances }
    }
}
