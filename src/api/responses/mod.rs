use std::time::SystemTime;

use models::*;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SellOrderResponse {
    pub id: BlockchainTransactionId,
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
    pub to: AccountAddress,
    pub from: AccountAddress,
}

impl From<SellOrder> for SellOrderResponse {
    fn from(sell: SellOrder) -> Self {
        Self {
            id: sell.id,
            from_currency: sell.from_currency,
            to_currency: sell.to_currency,
            amount: sell.amount,
            rate: sell.rate,
            to: sell.to,
            from: sell.from,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeResponse {
    pub id: ExchangeId,
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub reserved_for: i32,
    pub rate: f64,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl From<Exchange> for ExchangeResponse {
    fn from(ex: Exchange) -> Self {
        Self {
            id: ex.id,
            from_currency: ex.from_currency,
            to_currency: ex.to_currency,
            amount: ex.amount,
            reserved_for: ex.reserved_for,
            rate: ex.rate,
            created_at: ex.created_at,
            updated_at: ex.updated_at,
        }
    }
}
