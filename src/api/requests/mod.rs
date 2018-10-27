use models::*;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostExchangeRequest {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
    pub rate: f64,
    pub to: AccountAddress,
    pub from: AccountAddress,
}

impl From<PostExchangeRequest> for CreateSellOrder {
    fn from(req: PostExchangeRequest) -> Self {
        Self {
            from_currency: req.from_currency,
            to_currency: req.to_currency,
            amount: req.amount,
            rate: req.rate,
            to: req.to,
            from: req.from,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetExchangeParams {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub amount: Amount,
}

impl From<GetExchangeParams> for ExchangeRequest {
    fn from(req: GetExchangeParams) -> Self {
        Self {
            from_currency: req.from_currency,
            to_currency: req.to_currency,
            amount: req.amount,
        }
    }
}
