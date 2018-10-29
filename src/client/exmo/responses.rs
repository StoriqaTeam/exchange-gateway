use models::*;

#[derive(Debug, Clone)]
pub struct ExmoCreateSellOrder {
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl From<Exchange> for ExmoCreateSellOrder {
    fn from(sell: Exchange) -> Self {
        Self {
            from: sell.from_,
            to: sell.to_,
            amount: sell.amount,
            rate: sell.rate,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoSellOrderResponse {
    pub id: BlockchainTransactionId,
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl From<ExmoSellOrderResponse> for SellOrder {
    fn from(sell: ExmoSellOrderResponse) -> Self {
        Self {
            id: sell.id,
            from: sell.from,
            to: sell.to,
            amount: sell.amount,
            rate: sell.rate,
        }
    }
}

impl Default for ExmoSellOrderResponse {
    fn default() -> Self {
        Self {
            id: BlockchainTransactionId::default(),
            from: Currency::Eth,
            to: Currency::Btc,
            amount: Amount::default(),
            rate: f64::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExmoGetExchangeResponse {
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl Default for ExmoGetExchangeResponse {
    fn default() -> Self {
        Self {
            from: Currency::Eth,
            to: Currency::Btc,
            amount: Amount::default(),
            rate: 0.33,
        }
    }
}
