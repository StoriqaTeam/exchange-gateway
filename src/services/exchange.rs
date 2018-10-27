use std::sync::Arc;

use super::*;
use client::ExmoClient;
use models::*;
use prelude::*;
use repos::{DbExecutor, ExchangesRepo};

#[derive(Clone)]
pub struct ExchangeServiceImpl<E: DbExecutor> {
    exchange_repo: Arc<dyn ExchangesRepo>,
    db_executor: E,
    exmo_client: Arc<dyn ExmoClient>,
    reserved_for: i32,
}

impl<E: DbExecutor> ExchangeServiceImpl<E> {
    pub fn new(exchange_repo: Arc<ExchangesRepo>, db_executor: E, exmo_client: Arc<dyn ExmoClient>, reserved_for: i32) -> Self {
        Self {
            exchange_repo,
            db_executor,
            exmo_client,
            reserved_for,
        }
    }
}

pub trait ExchangeService: Send + Sync + 'static {
    fn sell(&self, input: CreateSellOrder) -> ServiceFuture<SellOrder>;
    fn get_exchange(&self, input: ExchangeRequest) -> ServiceFuture<Exchange>;
}

impl<E: DbExecutor> ExchangeService for ExchangeServiceImpl<E> {
    fn sell(&self, input: CreateSellOrder) -> ServiceFuture<SellOrder> {
        let exmo_client = self.exmo_client.clone();
        let input_clone = input.clone();
        let db_executor = self.db_executor.clone();
        let exchange_repo = self.exchange_repo.clone();
        Box::new(
            db_executor
                .execute(move || {
                    let input_clone = input.clone();
                    let input_clone2 = input.clone();
                    exchange_repo
                        .get(input.into())
                        .map_err(ectx!(try convert => input_clone))?
                        .ok_or_else(|| ectx!(err ErrorContext::NoExchangeRate, ErrorKind::NotFound => input_clone2))
                }).and_then(move |_| {
                    exmo_client
                        .sell(input_clone.clone().into())
                        .map_err(ectx!(convert => input_clone))
                        .map(|s| s.into())
                }),
        )
    }

    fn get_exchange(&self, input: ExchangeRequest) -> ServiceFuture<Exchange> {
        let exchange_repo = self.exchange_repo.clone();
        let db_executor = self.db_executor.clone();
        let exmo_client = self.exmo_client.clone();
        let reserved_for = self.reserved_for;
        let input_clone = input.clone();
        Box::new(
            exmo_client
                .get_rate(input.clone())
                .map_err(ectx!(convert => input))
                .map(|resp| resp.rate)
                .and_then(move |rate| {
                    db_executor.execute(move || {
                        let new_exchange = NewExchange::new(input_clone, reserved_for, rate);
                        exchange_repo
                            .create(new_exchange.clone())
                            .map_err(ectx!(ErrorKind::Internal => new_exchange))
                    })
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::*;
    use repos::*;
    use tokio_core::reactor::Core;

    fn sell_service() -> ExchangeServiceImpl<DbExecutorMock> {
        let exchange_repo = Arc::new(ExchangesRepoMock::default());
        let exmo_client = Arc::new(ExmoClientMock::default());
        let db_executor = DbExecutorMock::default();
        let reserved_for = 600;
        ExchangeServiceImpl::new(exchange_repo, db_executor, exmo_client, reserved_for)
    }

    #[test]
    fn test_exchange_sell() {
        let mut core = Core::new().unwrap();
        let service = sell_service();
        let new_exchange = CreateSellOrder::default();
        let exchange = core.run(service.sell(new_exchange));
        assert!(exchange.is_ok());
    }
    #[test]
    fn test_exchange_get() {
        let mut core = Core::new().unwrap();
        let service = sell_service();
        let new_exchange = ExchangeRequest::default();
        let exchange = core.run(service.get_exchange(new_exchange));
        assert!(exchange.is_ok());
    }
}
