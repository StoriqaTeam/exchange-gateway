use std::sync::Arc;
use std::time::{Duration, SystemTime};

use futures::future::{self, Either};

use super::auth::AuthService;
use super::*;
use client::ExmoClient;
use models::*;
use prelude::*;
use repos::{DbExecutor, ExchangesRepo};

#[derive(Clone)]
pub struct ExchangeServiceImpl<E: DbExecutor> {
    auth_service: Arc<dyn AuthService>,
    exchange_repo: Arc<dyn ExchangesRepo>,
    db_executor: E,
    exmo_client: Arc<dyn ExmoClient>,
    expiration: u64,
    rate_upside: f64,
    safety_threshold: f64,
}

impl<E: DbExecutor> ExchangeServiceImpl<E> {
    pub fn new(
        auth_service: Arc<AuthService>,
        exchange_repo: Arc<ExchangesRepo>,
        db_executor: E,
        exmo_client: Arc<dyn ExmoClient>,
        expiration: u64,
        rate_upside: f64,
        safety_threshold: f64,
    ) -> Self {
        Self {
            auth_service,
            exchange_repo,
            db_executor,
            exmo_client,
            expiration,
            rate_upside,
            safety_threshold,
        }
    }
}

pub trait ExchangeService: Send + Sync + 'static {
    fn sell(&self, token: AuthenticationToken, input: CreateSellOrder) -> ServiceFuture<SellOrder>;
    fn get_rate(&self, token: AuthenticationToken, input: GetRate) -> ServiceFuture<Exchange>;
}

impl<E: DbExecutor> ExchangeService for ExchangeServiceImpl<E> {
    fn sell(&self, token: AuthenticationToken, input: CreateSellOrder) -> ServiceFuture<SellOrder> {
        let exmo_client = self.exmo_client.clone();
        let input_clone = input.clone();
        let db_executor = self.db_executor.clone();
        let exchange_repo = self.exchange_repo.clone();
        Box::new(self.auth_service.authenticate(token).and_then(move |user| {
            db_executor
                .execute(move || {
                    let input_clone = input.clone();
                    let input_clone2 = input.clone();
                    exchange_repo
                        .get(input.into())
                        .map_err(ectx!(try convert => input_clone))?
                        .ok_or_else(|| ectx!(err ErrorContext::NoExchangeRate, ErrorKind::NotFound => input_clone2))
                }).and_then(move |rate| {
                    if rate.user_id != user.id {
                        Either::A(future::err(
                            ectx!(err ErrorContext::InvalidToken, ErrorKind::Unauthorized => user.id),
                        ))
                    } else {
                        Either::B(
                            exmo_client
                                .sell(rate.into())
                                .map_err(ectx!(convert => input_clone))
                                .map(|s| s.into()),
                        )
                    }
                })
        }))
    }

    fn get_rate(&self, token: AuthenticationToken, input: GetRate) -> ServiceFuture<Exchange> {
        let exchange_repo = self.exchange_repo.clone();
        let db_executor = self.db_executor.clone();
        let exmo_client = self.exmo_client.clone();
        let expiration = SystemTime::now() + Duration::from_secs(self.expiration);
        let input_clone = input.clone();
        Box::new(self.auth_service.authenticate(token).and_then(move |user| {
            exmo_client
                .get_current_rate(input.clone())
                .map_err(ectx!(convert => input))
                .and_then(move |rate| {
                    db_executor.execute(move || {
                        let new_exchange = NewExchange::new(input_clone, expiration, rate, user.id);
                        exchange_repo
                            .create(new_exchange.clone())
                            .map_err(ectx!(ErrorKind::Internal => new_exchange))
                    })
                })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::*;
    use repos::*;
    use tokio_core::reactor::Core;

    fn create_sell_service(token: AuthenticationToken, user_id: UserId) -> ExchangeServiceImpl<DbExecutorMock> {
        let auth_service = Arc::new(AuthServiceMock::new(vec![(token, user_id)]));
        let exchange_repo = Arc::new(ExchangesRepoMock::default());
        let exmo_client = Arc::new(ExmoClientMock::default());
        let db_executor = DbExecutorMock::default();
        let reserved_for = 600;
        let rate_upside = 0f64;
        let safety_threshold = 0f64;
        ExchangeServiceImpl::new(
            auth_service,
            exchange_repo,
            db_executor,
            exmo_client,
            reserved_for,
            rate_upside,
            safety_threshold,
        )
    }

    #[test]
    fn test_exchange_sell() {
        let mut core = Core::new().unwrap();
        let token = AuthenticationToken::default();
        let user_id = UserId::generate();
        let service = create_sell_service(token.clone(), user_id);
        let new_exchange = CreateSellOrder::default();
        let exchange = core.run(service.sell(token, new_exchange));
        assert!(exchange.is_ok());
    }
    #[test]
    fn test_exchange_get() {
        let mut core = Core::new().unwrap();
        let token = AuthenticationToken::default();
        let user_id = UserId::generate();
        let service = create_sell_service(token.clone(), user_id);
        let new_exchange = GetRate::default();
        let exchange = core.run(service.get_rate(token, new_exchange));
        assert!(exchange.is_ok());
    }
}
