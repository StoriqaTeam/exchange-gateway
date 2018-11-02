use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use futures::future::{self, Either};
use futures::stream::iter_ok;
use tokio_core::reactor::Core;

use super::auth::AuthService;
use super::*;
use client::ExmoClient;
use models::*;
use prelude::*;
use repos::{DbExecutor, ExchangesRepo, Isolation, SellOrdersRepo};
use utils::{get_exmo_type, need_revert};

#[derive(Clone)]
pub struct ExchangeServiceImpl<E: DbExecutor> {
    auth_service: Arc<dyn AuthService>,
    exchange_repo: Arc<dyn ExchangesRepo>,
    sell_orders_repo: Arc<dyn SellOrdersRepo>,
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
        sell_orders_repo: Arc<dyn SellOrdersRepo>,
        db_executor: E,
        exmo_client: Arc<dyn ExmoClient>,
        expiration: u64,
        rate_upside: f64,
        safety_threshold: f64,
    ) -> Self {
        Self {
            auth_service,
            exchange_repo,
            sell_orders_repo,
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
    fn start_selling(&self, exchange: Exchange, actual_amount: Amount) -> ServiceFuture<SellOrder>;
    fn get_rate(&self, token: AuthenticationToken, input: GetRate) -> ServiceFuture<Exchange>;
    fn get_current_rate(&self, from: Currency, to: Currency, amount: Amount) -> ServiceFuture<f64>;
}

impl<E: DbExecutor> ExchangeService for ExchangeServiceImpl<E> {
    fn sell(&self, token: AuthenticationToken, input: CreateSellOrder) -> ServiceFuture<SellOrder> {
        let db_executor = self.db_executor.clone();
        let exchange_repo = self.exchange_repo.clone();
        let safety_threshold = self.safety_threshold;
        let input_clone2 = input.clone();
        let from = input.from;
        let to = input.to;
        let service = self.clone();
        let service2 = self.clone();
        let amount = input.actual_amount;
        Box::new(self.auth_service.authenticate(token).and_then(move |user| {
            db_executor
                .execute(move || {
                    let input_clone = input.clone();
                    let input_clone2 = input.clone();
                    exchange_repo
                        .get(input.into())
                        .map_err(ectx!(try convert => input_clone))?
                        .ok_or_else(|| ectx!(err ErrorContext::NoExchangeRate, ErrorKind::NotFound => input_clone2))
                }).and_then(move |exchange| {
                    if exchange.user_id != user.id {
                        Either::A(future::err(
                            ectx!(err ErrorContext::InvalidToken, ErrorKind::Unauthorized => user.id),
                        ))
                    } else {
                        let input_clone = input_clone2.clone();
                        let users_rate = exchange.rate;
                        Either::B(service.get_current_rate(from, to, amount).and_then(move |current_rate| {
                            let safety_rate = current_rate / (1f64 + safety_threshold);
                            // we recalculate current_rate with safety_threshold, for not to loose on conversion, example:
                            // if current_rate is 10, rate_for_user (exchange.rate) is 9, safety threshold = 0,05:
                            // then safety_rate = 10 / 1,05, it is still higher then rate for user - we don't loose, Ok!
                            // if current_rate is 9, rate_for_user (exchange.rate) is 9, safety threshold = 0,05:
                            // then safety_rate = 9 / 1,05, it is lower then rate for user - we loose, Error!
                            if safety_rate > users_rate {
                                Either::A(service2.start_selling(exchange, input_clone.actual_amount))
                            } else {
                                Either::B(future::err(
                                    ectx!(err ErrorContext::NoSuchRate, ErrorKind::Internal => safety_rate, users_rate),
                                ))
                            }
                        }))
                    }
                })
        }))
    }

    fn get_current_rate(&self, from: Currency, to: Currency, amount: Amount) -> ServiceFuture<f64> {
        let exmo_client = self.exmo_client.clone();
        let amount = from.to_f64(amount);
        let currencies_exchange = get_exmo_type(from, to);
        Box::new(
            iter_ok::<_, Error>(currencies_exchange)
                .fold((amount, 1f64), move |(quantity, rate), currency_exchange| {
                    let (pair, order_type) = currency_exchange;
                    exmo_client
                        .get_book(pair)
                        .and_then(move |book| book.get_rate(quantity, order_type))
                        .map_err(ectx!(convert => from, to, quantity))
                        .and_then(move |mut res| {
                            if need_revert(order_type) {
                                res = 1f64 / res;
                            };
                            let new_rate = rate * res;
                            let new_quantity = new_rate * quantity;
                            Ok((new_quantity, new_rate)) as Result<(f64, f64), Error>
                        })
                }).map(|(_, rate)| rate),
        )
    }

    fn get_rate(&self, token: AuthenticationToken, input: GetRate) -> ServiceFuture<Exchange> {
        let exchange_repo = self.exchange_repo.clone();
        let db_executor = self.db_executor.clone();
        let rate_upside = self.rate_upside;
        let expiration = SystemTime::now() + Duration::from_secs(self.expiration);
        let input_clone = input.clone();
        let amount = input.amount;
        let from = input.from;
        let to = input.to;
        let service = self.clone();
        Box::new(self.auth_service.authenticate(token).and_then(move |user| {
            service.get_current_rate(from, to, amount).and_then(move |rate| {
                db_executor.execute(move || {
                    // we recalculate rate with rate_upside, for not to loose on conversion, example:
                    // if rate is 10 - it means that for 1 BTC one will receive 10 ETH
                    // for not to lose we give him for 1 BTC - 9 ETH
                    // if rate is 0,1 - it means that for 1 ETH one will receive 0,1 BTC
                    // for not to lose we give him for 1 ETH - 0,09 ETH
                    // rate_upside must be > 0
                    let rate_with_upside = rate / (1f64 + rate_upside);
                    let new_exchange = NewExchange::new(input_clone, expiration, rate_with_upside, user.id);
                    exchange_repo
                        .create(new_exchange.clone())
                        .map_err(ectx!(ErrorKind::Internal => new_exchange))
                })
            })
        }))
    }

    fn start_selling(&self, exchange: Exchange, quantity: Amount) -> ServiceFuture<SellOrder> {
        let exmo_client = self.exmo_client.clone();
        let db_executor = self.db_executor.clone();
        let sell_orders_repo = self.sell_orders_repo.clone();
        let from = exchange.from_;
        let to = exchange.to_;
        Box::new(
            db_executor
                .execute_transaction_with_isolation(Isolation::Serializable, move || {
                    let mut core = Core::new().unwrap();
                    get_exmo_type(from, to)
                        .into_iter()
                        .try_fold(from.to_f64(quantity), move |quantity, currency_exchange| {
                            let (pair, order_type) = currency_exchange;
                            let pair_clone = pair.clone();
                            let data = Some(json!({"quantity": quantity, "pair": pair, "order_type": order_type ,"status": "Creating order"}));
                            let nonce = sell_orders_repo
                                .create(NewSellOrder::new(data.clone()))
                                .map_err(ectx!(try convert => data))
                                .map(|c| c.id)?;
                            let order_id = core.run(exmo_client
                                .create_order(pair.clone(), quantity, order_type, nonce.inner())
                                .map_err(ectx!(try convert => pair, quantity, order_type, nonce)))
                                ?;

                            thread::sleep(Duration::from_millis(350));

                            // let data = Some(json!({"quantity": quantity, "pair": pair_clone, "order_id": order_id ,"status": "Getting Order info"}));
                            let data = Some(json!({"quantity": quantity, "pair": pair_clone,"status": "Getting Order info"}));
                            let nonce = sell_orders_repo
                                .create(NewSellOrder::new(data.clone()))
                                .map_err(ectx!(try convert => data))
                                .map(|c| c.id)?;
                            
                            let r : Result<(f64, f64), Error>= core.run( exmo_client
                                .get_order_status(order_id, nonce.inner())
                                .map_err(ectx!(convert => order_id, nonce)))
                                ;

                            debug!("get_order_status - {:?}", r);

                            thread::sleep(Duration::from_millis(350));
                            let data = Some(json!({"quantity": quantity, "pair": pair_clone,"status": "Getting Order info"}));
                            let nonce = sell_orders_repo
                                .create(NewSellOrder::new(data.clone()))
                                .map_err(ectx!(try convert => data))
                                .map(|c| c.id)?;

                            let r : Result<(f64, f64), Error> = core.run(exmo_client
                                .canceled_orders(nonce.inner())
                                .map_err(ectx!( convert => nonce)));

                            debug!("canceled_orders - {:?}", r);

                            thread::sleep(Duration::from_millis(350));
                            let data = Some(json!({"quantity": quantity, "pair": pair_clone,"status": "Getting Order info"}));
                            let nonce = sell_orders_repo
                                .create(NewSellOrder::new(data.clone()))
                                .map_err(ectx!(try convert => data))
                                .map(|c| c.id)?;
                            
                            let r : Result<(f64, f64), Error> = core.run( exmo_client
                                .user_open_orders(nonce.inner())
                                .map_err(ectx!( convert => nonce)));
                            
                            debug!("user_open_orders - {:?}", r);

                            thread::sleep(Duration::from_millis(350));
                            let data = Some(json!({"quantity": quantity, "pair": pair_clone,"status": "Getting Order info"}));
                            let nonce = sell_orders_repo
                                .create(NewSellOrder::new(data.clone()))
                                .map_err(ectx!(try convert => data))
                                .map(|c| c.id)?;
                            
                            let r : Result<(f64, f64), Error> = core.run( exmo_client
                                .get_user_trades("STQ_BTC".to_string(),nonce.inner())
                                .map_err(ectx!( convert => nonce)))
                                ;

                            debug!("get_user_trades STQ_BTC - {:?}", r);
                           
                            thread::sleep(Duration::from_millis(350));
                            let data = Some(json!({"quantity": quantity, "pair": pair_clone,"status": "Getting Order info"}));
                            let nonce = sell_orders_repo
                                .create(NewSellOrder::new(data.clone()))
                                .map_err(ectx!(try convert => data))
                                .map(|c| c.id)?;
                            
                            let (in_amount, out_amount) = core.run( exmo_client
                                .get_user_trades("ETH_BTC".to_string(),nonce.inner())
                                .map_err(ectx!(try convert => nonce)))
                                ?;

                            debug!("get_user_trades ETH_BTC - {:?}", r);

                            // let data = Some(json!({"in_amount": in_amount, "out_amount": out_amount, "pair": pair_clone, "order_id": order_id ,"status": "Order info"}));
                            thread::sleep(Duration::from_millis(350));
                            let data = Some(json!({"in_amount": in_amount, "out_amount": out_amount, "pair": pair_clone,"status": "Order info"}));
                            let _ = sell_orders_repo
                                .create(NewSellOrder::new(data.clone()));

                            Ok(out_amount)
                        })
                }).map(move |actual_quantity| SellOrder::new(actual_quantity, from, to)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::*;
    use models::{BTC_DECIMALS, ETH_DECIMALS, STQ_DECIMALS};
    use repos::*;
    use tokio_core::reactor::Core;

    fn create_sell_service(token: AuthenticationToken, user_id: UserId) -> ExchangeServiceImpl<DbExecutorMock> {
        let auth_service = Arc::new(AuthServiceMock::new(vec![(token, user_id)]));
        let exchange_repo = Arc::new(ExchangesRepoMock::default());
        let sell_orders_repo = Arc::new(SellOrdersRepoMock::default());
        let exmo_client = Arc::new(ExmoClientMock::default());
        let db_executor = DbExecutorMock::default();
        let reserved_for = 600;
        let rate_upside = 0f64;
        let safety_threshold = 0f64;
        ExchangeServiceImpl::new(
            auth_service,
            exchange_repo,
            sell_orders_repo,
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

    #[test]
    fn test_rates() {
        let mut core = Core::new().unwrap();
        let token = AuthenticationToken::default();
        let user_id = UserId::generate();
        let service = create_sell_service(token.clone(), user_id);

        let rate = core.run(service.get_current_rate(Currency::Eth, Currency::Btc, Amount::new(ETH_DECIMALS)));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Btc, Currency::Eth, Amount::new(BTC_DECIMALS)));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Stq, Currency::Btc, Amount::new(STQ_DECIMALS)));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Btc, Currency::Stq, Amount::new(BTC_DECIMALS)));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Stq, Currency::Eth, Amount::new(STQ_DECIMALS)));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Eth, Currency::Stq, Amount::new(ETH_DECIMALS)));
        assert!(rate.is_ok());
    }
}
