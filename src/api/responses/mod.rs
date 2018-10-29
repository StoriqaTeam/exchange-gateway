use std::time::SystemTime;

use models::*;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsersResponse {
    pub id: UserId,
    pub name: String,
    pub authentication_token: AuthenticationToken,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
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
    pub actual_rate: f64,
}

impl From<SellOrder> for SellOrderResponse {
    fn from(sell: SellOrder) -> Self {
        Self {
            from: sell.from,
            to: sell.to,
            amount: sell.amount,
            actual_rate: sell.rate,
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
    pub expiration: SystemTime,
    pub rate: f64,
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
        }
    }
}
