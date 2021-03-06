use models::*;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostUsersRequest {
    pub id: UserId,
    pub name: String,
    pub authentication_token: AuthenticationToken,
}

impl From<PostUsersRequest> for NewUser {
    fn from(req: PostUsersRequest) -> Self {
        Self {
            id: req.id,
            name: req.name,
            authentication_token: req.authentication_token,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PutUsersRequest {
    pub name: Option<String>,
    pub authentication_token: Option<AuthenticationToken>,
}

impl From<PutUsersRequest> for UpdateUser {
    fn from(req: PutUsersRequest) -> Self {
        Self {
            name: req.name,
            authentication_token: req.authentication_token,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostExchangeRequest {
    pub id: ExchangeId,
    pub from: Currency,
    pub to: Currency,
    pub actual_amount: Amount,
    pub amount_currency: Currency,
}

impl From<PostExchangeRequest> for CreateSellOrder {
    fn from(req: PostExchangeRequest) -> Self {
        Self {
            id: req.id,
            from: req.from,
            to: req.to,
            actual_amount: req.actual_amount,
            amount_currency: req.amount_currency,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostRateRequest {
    pub id: ExchangeId,
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub amount_currency: Currency,
}

impl From<PostRateRequest> for GetRate {
    fn from(req: PostRateRequest) -> Self {
        Self {
            id: req.id,
            from: req.from,
            to: req.to,
            amount: req.amount,
            amount_currency: req.amount_currency,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PostRateRefreshRequest {
    pub exchange_id: ExchangeId,
}

impl From<PostRateRefreshRequest> for ExchangeId {
    fn from(req: PostRateRefreshRequest) -> Self {
        req.exchange_id
    }
}
