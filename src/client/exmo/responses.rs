use models::*;

#[derive(Debug, Clone)]
pub struct ExmoCreateSellOrder {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
    pub to: AccountAddress,
    pub from: AccountAddress,
}

impl From<CreateSellOrder> for ExmoCreateSellOrder {
    fn from(sell: CreateSellOrder) -> Self {
        Self {
            from_currency: sell.from_currency,
            to_currency: sell.to_currency,
            amount: sell.amount,
            rate: sell.rate,
            to: sell.to,
            from: sell.from,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoSellOrderResponse {
    pub id: BlockchainTransactionId,
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
    pub to: AccountAddress,
    pub from: AccountAddress,
}

impl From<ExmoSellOrderResponse> for SellOrder {
    fn from(sell: ExmoSellOrderResponse) -> Self {
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

impl Default for ExmoSellOrderResponse {
    fn default() -> Self {
        Self {
            id: BlockchainTransactionId::default(),
            from_currency: Currency::Eth,
            to_currency: Currency::Btc,
            amount: Amount::default(),
            rate: f64::default(),
            to: AccountAddress::default(),
            from: AccountAddress::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExmoGetExchangeResponse {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl Default for ExmoGetExchangeResponse {
    fn default() -> Self {
        Self {
            from_currency: Currency::Eth,
            to_currency: Currency::Btc,
            amount: Amount::default(),
            rate: 0.33,
        }
    }
}
