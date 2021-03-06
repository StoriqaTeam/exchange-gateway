use std::sync::Arc;
use std::thread;
use time::Duration;

use failure::Fail;
use futures::future::{self, Either};
use futures::stream::iter_ok;
use futures_retry::FutureRetry;
use tokio_core::reactor::Core;
use validator::{ValidationError, ValidationErrors};

use super::auth::AuthService;
use super::*;
use client::ExmoClient;
use config::{Config, CurrenciesLimits};

use models::*;
use prelude::*;
use repos::{DbExecutor, ExchangesRepo, Isolation, SellOrdersRepo};
use utils::{get_exmo_type, need_revert};

const RETRY_ATTEMPTS: usize = 3;
const RETRY_TIMEOUT: u64 = 1;

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
    limits: CurrenciesLimits,
    retry_attempts: usize,
    retry_timeout: u64,
}

impl<E: DbExecutor> ExchangeServiceImpl<E> {
    pub fn new(
        auth_service: Arc<AuthService>,
        exchange_repo: Arc<ExchangesRepo>,
        sell_orders_repo: Arc<dyn SellOrdersRepo>,
        db_executor: E,
        exmo_client: Arc<dyn ExmoClient>,
        config: Config,
    ) -> Self {
        let expiration = config.exchange_options.expiration;
        let rate_upside = config.exchange_options.rate_upside;
        let safety_threshold = config.exchange_options.safety_threshold;
        let retry_attempts = config.client.retry_attempts.unwrap_or(RETRY_ATTEMPTS);
        let retry_timeout = config.client.retry_timeout.unwrap_or(RETRY_TIMEOUT);
        let limits = config.limits.clone();
        Self {
            auth_service,
            exchange_repo,
            sell_orders_repo,
            db_executor,
            exmo_client,
            expiration,
            rate_upside,
            safety_threshold,
            limits,
            retry_attempts,
            retry_timeout,
        }
    }

    // TODO: remove the use of Core
    fn check_balance(&self, from: Currency, amount: Amount, amount_currency: Currency, current_rate: f64) -> ServiceFuture<()> {
        let exmo_client = self.exmo_client.clone();
        let db_executor = self.db_executor.clone();
        let sell_orders_repo = self.sell_orders_repo.clone();

        let needed_quantity = if amount_currency == from {
            amount_currency.to_f64(amount)
        } else {
            amount_currency.to_f64(amount) / current_rate
        };
        let currency = from;
        Box::new(db_executor.execute_transaction_with_isolation(Isolation::Serializable, move || {
            let mut core = Core::new().unwrap();
            let mut e: Error= ectx!(err ErrorContext::Internal, ErrorKind::Internal);
            for _ in 0..3 {
                let data = Some(json!({"currency": currency, "needed_quantity": needed_quantity, "status": "Check user balance on exmo"}));
                sell_orders_repo
                    .create(NewSellOrder::new(data.clone()))
                    .map_err(ectx!(try convert => data))
                    .map(|c| c.id)?;
                let nonce = Nonce::generate();

                let user_info = core.run(exmo_client.get_user_balances(nonce).map_err(ectx!(convert => nonce)));
                match user_info {
                    Ok(user_info) => {
                        let users_balance = user_info.get_balance(currency);
                        if users_balance < needed_quantity {
                            return Err(ectx!(err ErrorContext::NotEnoughCurrencyBalance, ErrorKind::Internal => users_balance, needed_quantity, currency));
                        } else {
                            return Ok(());
                        }
                    }
                    Err(err) => {
                        thread::sleep(Duration::milliseconds(200).to_std().unwrap());
                        e = err;
                    }
                }
            }
            Err(e)
        }))
    }

    fn get_current_rate(&self, from: Currency, to: Currency, amount: Amount, amount_currency: Currency) -> ServiceFuture<f64> {
        let exmo_client = self.exmo_client.clone();
        let retry_attempts = self.retry_attempts;
        let retry_timeout = self.retry_timeout;
        let currencies_exchange = get_exmo_type(from, to, amount_currency);
        Box::new(
            self.recalc_default_quantity(from, to, amount, amount_currency)
                .and_then(move |start_quantity| {
                    iter_ok::<_, Error>(currencies_exchange)
                        .fold(
                            (amount_currency.to_f64(start_quantity), 1f64),
                            move |(quantity, rate), currency_exchange| {
                                let (pair, order_type) = currency_exchange;
                                let pair_clone = pair.clone();
                                let exmo_client = exmo_client.clone();
                                FutureRetry::new(
                                    move || exmo_client.get_book(pair_clone.clone()),
                                    RetryErrorHandler::new(retry_attempts, retry_timeout),
                                )
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
                            },
                        )
                        .map(|(_, rate)| rate)
                }),
        )
    }

    fn create_rate(&self, input: GetRate, user_id: UserId) -> ServiceFuture<Exchange> {
        let exchange_repo = self.exchange_repo.clone();
        let db_executor = self.db_executor.clone();
        let rate_upside = self.rate_upside;
        let expiration = ::chrono::Utc::now().naive_utc() + Duration::seconds(self.expiration as i64);
        let input_clone = input.clone();
        let amount = input.amount;
        let from = input.from;
        let to = input.to;
        let service = self.clone();
        let amount_currency = input.amount_currency;

        Box::new(service.get_current_rate(from, to, amount, amount_currency).and_then(move |rate| {
            db_executor.execute(move || {
                // we recalculate rate with rate_upside, for not to lose on conversion, example:
                // if rate is 10 - it means that for 1 BTC one will receive 10 ETH
                // for not to lose we give him for 1 BTC - 9 ETH
                // if rate is 0,1 - it means that for 1 ETH one will receive 0,1 BTC
                // for not to lose we give him for 1 ETH - 0,09 ETH
                // rate_upside must be > 0
                let rate_with_upside = rate * (1f64 - rate_upside);
                let new_exchange = NewExchange::new(input_clone, expiration, rate_with_upside, user_id);
                exchange_repo
                    .create(new_exchange.clone())
                    .map_err(ectx!(ErrorKind::Internal => new_exchange))
            })
        }))
    }

    /// conversion from eth to stq is done through usd,
    /// though when amount_currency is equal to `to` we first need to know how much USD we need to buy
    fn recalc_default_quantity(&self, from: Currency, to: Currency, amount: Amount, amount_currency: Currency) -> ServiceFuture<Amount> {
        let retry_attempts = self.retry_attempts;
        let retry_timeout = self.retry_timeout;
        let (pair, order_type) = match (from, to, amount_currency) {
            (Currency::Eth, Currency::Stq, Currency::Stq) => ("STQ_USD".to_string(), OrderType::SellTotal),
            (Currency::Stq, Currency::Eth, Currency::Eth) => ("ETH_USD".to_string(), OrderType::SellTotal),
            _ => return Box::new(future::ok(amount)),
        };

        let exmo_client = self.exmo_client.clone();
        let quantity = amount_currency.to_f64(amount);
        let pair_clone = pair.clone();
        Box::new(
            FutureRetry::new(
                move || exmo_client.get_book(pair_clone.clone()),
                RetryErrorHandler::new(retry_attempts, retry_timeout),
            )
            .and_then(move |book| book.get_rate(quantity, order_type))
            .map_err(ectx!(convert => from, to, quantity))
            .and_then(move |mut rate| {
                if need_revert(order_type) {
                    rate = 1f64 / rate;
                };
                let new_quantity = rate * quantity;
                Ok(amount_currency.from_f64(new_quantity)) as Result<Amount, Error>
            }),
        )
    }

    fn start_selling(&self, exchange: Exchange, quantity: Amount) -> ServiceFuture<SellOrder> {
        let exmo_client = self.exmo_client.clone();
        let db_executor = self.db_executor.clone();
        let sell_orders_repo = self.sell_orders_repo.clone();
        let from = exchange.from_;
        let to = exchange.to_;
        let amount_currency = exchange.amount_currency;
        Box::new(self.recalc_default_quantity(from, to, quantity, amount_currency).and_then(move |start_quantity|{
            db_executor
                .execute_transaction_with_isolation(Isolation::Serializable, move || {
                    let mut core = Core::new().unwrap();
                    get_exmo_type(from, to, amount_currency)
                        .into_iter()
                        .try_fold(amount_currency.to_f64(start_quantity), move |quantity, currency_exchange| {
                            let (pair, order_type) = currency_exchange;
                            let mut e: Error= ectx!(err ErrorContext::Internal, ErrorKind::Internal);
                            for _ in 0..3 {
                                let pair_clone = pair.clone();
                                let pair_clone2 = pair.clone();
                                let data = Some(json!({"quantity": quantity, "pair": pair_clone, "order_type": order_type ,"status": "Creating order"}));
                                sell_orders_repo
                                    .create(NewSellOrder::new(data.clone()))
                                    .map_err(ectx!(try convert => data))
                                    .map(|c| c.id)?;

                                let nonce = Nonce::generate();
                                let order_id = core.run(exmo_client
                                    .create_order(pair_clone.clone(), quantity, order_type, nonce)
                                    .map_err(ectx!(try convert => pair_clone, quantity, order_type, nonce)))
                                    ?;

                                thread::sleep(Duration::milliseconds(200).to_std().unwrap());

                                let data = Some(json!({"quantity": quantity, "pair": pair_clone2, "order_id": order_id ,"status": "Getting Order info"}));
                                sell_orders_repo
                                    .create(NewSellOrder::new(data.clone()))
                                    .map_err(ectx!(try convert => data))
                                    .map(|c| c.id)?;

                                let nonce = Nonce::generate();
                                let order_status: Result<_, Error> = core.run( exmo_client
                                    .get_order_status(order_id, nonce)
                                    .map_err(ectx!(convert => order_id, nonce)));

                                match order_status {
                                    Ok((in_amount, out_amount)) => {
                                        let data = Some(json!({"in_amount": in_amount, "out_amount": out_amount, "pair": pair_clone2, "order_id": order_id ,"status": "Order info"}));
                                        let _ = sell_orders_repo
                                            .create(NewSellOrder::new(data.clone()));
                                        return Ok(in_amount);
                                    }
                                    Err(err) => {
                                        thread::sleep(Duration::milliseconds(200).to_std().unwrap());
                                        e = err;
                                    }
                                }
                            }
                            Err(e)

                        })
                }).map(move |actual_quantity| SellOrder::new(actual_quantity, from, to))
        })
        )
    }
}

pub trait ExchangeService: Send + Sync + 'static {
    fn sell(&self, token: AuthenticationToken, input: CreateSellOrder) -> ServiceFuture<SellOrder>;
    fn get_rate(&self, token: AuthenticationToken, input: GetRate) -> ServiceFuture<Exchange>;
    fn refresh_rate(&self, token: AuthenticationToken, exchange_id: ExchangeId) -> ServiceFuture<ExchangeRefresh>;
    fn metrics(&self) -> ServiceFuture<Metrics>;
}

impl<E: DbExecutor> ExchangeService for ExchangeServiceImpl<E> {
    fn sell(&self, token: AuthenticationToken, input: CreateSellOrder) -> ServiceFuture<SellOrder> {
        let db_executor = self.db_executor.clone();
        let exchange_repo = self.exchange_repo.clone();
        let safety_threshold = self.safety_threshold;
        let limits = self.limits.clone();
        let input_clone2 = input.clone();
        let from = input.from;
        let to = input.to;
        let service = self.clone();
        let service2 = self.clone();
        let service3 = self.clone();
        let amount = input.actual_amount;
        let amount_currency = input.amount_currency;
        Box::new(self.auth_service.authenticate(token).and_then(move |user| {
            validate(input.amount_currency, input.actual_amount, limits)
                .map_err(|e| ectx!(err e.clone(), ErrorKind::InvalidInput(e) => input))
                .into_future()
                .and_then(move |_| {
                    db_executor
                        .execute(move || {
                            let input_clone = input.clone();
                            let input_clone2 = input.clone();
                            exchange_repo
                                .get(input.into())
                                .map_err(ectx!(try convert => input_clone))?
                                .ok_or_else(|| {
                                    let mut errors = ValidationErrors::new();
                                    let mut error = ValidationError::new("expired");
                                    error.add_param("message".into(), &"exchange rate already expired".to_string());
                                    error.add_param("details".into(), &"no details".to_string());
                                    errors.add("exchange_rate", error);
                                    ectx!(err ErrorContext::NoExchangeRate, ErrorKind::InvalidInput(errors) => input_clone2)
                                })
                        })
                        .and_then(move |exchange| {
                            if exchange.user_id != user.id {
                                Either::A(future::err(
                                    ectx!(err ErrorContext::InvalidToken, ErrorKind::Unauthorized => user.id),
                                ))
                            } else {
                                let input_clone = input_clone2.clone();
                                let users_rate = exchange.rate;
                                Either::B(
                                    service
                                        .get_current_rate(from, to, amount, amount_currency)
                                        .and_then(move |current_rate| {
                                            let safety_rate = current_rate * (1f64 - safety_threshold);
                                            // we recalculate current_rate with safety_threshold, for not to lose on conversion, example:
                                            // if current_rate is 10, rate_for_user (exchange.rate) is 9, safety threshold = 0,05:
                                            // then safety_rate = 10 * 0.95 = 9.5, it is still higher than rate for user - we don't lose, Ok!
                                            // if current_rate is 9, rate_for_user (exchange.rate) is 9, safety threshold = 0,05:
                                            // then safety_rate = 9 * 0.95 = 8.55, it is lower than rate for user - we lose, Error!
                                            if safety_rate > users_rate {
                                                Either::A(
                                                    service2
                                                        .check_balance(from, amount, amount_currency, current_rate)
                                                        .and_then(move |_| service3.start_selling(exchange, input_clone.actual_amount)),
                                                )
                                            } else {
                                                Either::B(future::err(
                                                    ectx!(err ErrorContext::NoSuchRate, ErrorKind::Internal => safety_rate, users_rate),
                                                ))
                                            }
                                        }),
                                )
                            }
                        })
                })
        }))
    }

    fn get_rate(&self, token: AuthenticationToken, input: GetRate) -> ServiceFuture<Exchange> {
        let service = self.clone();

        Box::new(
            self.auth_service
                .authenticate(token)
                .and_then(move |user| service.create_rate(input, user.id)),
        )
    }

    fn refresh_rate(&self, token: AuthenticationToken, exchange_id: ExchangeId) -> ServiceFuture<ExchangeRefresh> {
        let exchange_repo = self.exchange_repo.clone();
        let exchange_repo2 = self.exchange_repo.clone();
        let db_executor = self.db_executor.clone();
        let safety_threshold = self.safety_threshold;
        let expiration = ::chrono::Utc::now().naive_utc() + Duration::seconds(self.expiration as i64);
        let service = self.clone();
        let service2 = self.clone();

        Box::new(self.auth_service.authenticate(token).and_then(move |user| {
            db_executor
                .execute(move || {
                    exchange_repo
                        .get_by_id(exchange_id)
                        .map_err(ectx!(try ErrorKind::Internal => exchange_id))?
                        .ok_or_else(|| {
                            let mut errors = ValidationErrors::new();
                            let mut error = ValidationError::new("not_found");
                            error.add_param("message".into(), &"exchange rate not found".to_string());
                            errors.add("exchange_rate", error);
                            ectx!(err ErrorContext::NoExchangeRate, ErrorKind::InvalidInput(errors) => exchange_id)
                        })
                })
                .and_then(move |exchange| {
                    let Exchange {
                        from_,
                        to_,
                        amount,
                        amount_currency,
                        ..
                    } = exchange;

                    service
                        .get_current_rate(from_.clone(), to_.clone(), amount.clone(), amount_currency.clone())
                        .map_err(ectx!(ErrorKind::Internal => from_, to_, amount, amount_currency))
                        .map(|current_rate| (exchange, current_rate))
                })
                .and_then(move |(exchange, current_rate)| {
                    let safety_rate = current_rate * (1f64 - safety_threshold);
                    let rate_for_user = exchange.rate;
                    if safety_rate > rate_for_user {
                        Either::A(db_executor.execute(move || {
                            let exchange_id = exchange.id.clone();
                            exchange_repo2
                                .update_expiration(exchange_id, expiration)
                                .map_err(ectx!(convert => exchange_id))
                                .map(|exchange| ExchangeRefresh {
                                    exchange,
                                    is_new_rate: false,
                                })
                        }))
                    } else {
                        let Exchange {
                            from_,
                            to_,
                            amount,
                            amount_currency,
                            ..
                        } = exchange;

                        let get_rate = GetRate {
                            id: ExchangeId::generate(),
                            from: from_,
                            to: to_,
                            amount,
                            amount_currency,
                        };

                        Either::B(service2.create_rate(get_rate, user.id).map(|exchange| ExchangeRefresh {
                            exchange,
                            is_new_rate: true,
                        }))
                    }
                })
        }))
    }

    fn metrics(&self) -> ServiceFuture<Metrics> {
        let exmo_client = self.exmo_client.clone();
        let db_executor = self.db_executor.clone();
        let sell_orders_repo = self.sell_orders_repo.clone();
        Box::new(db_executor.execute_transaction_with_isolation(Isolation::Serializable, move || {
            let mut core = Core::new().unwrap();
            let mut e: Error = ectx!(err ErrorContext::Internal, ErrorKind::Internal);
            for _ in 0..3 {
                let data = Some(json!({"status": "Monitor user balance on exmo"}));
                let nonce = sell_orders_repo
                    .create(NewSellOrder::new(data.clone()))
                    .map_err(ectx!(try convert => data))
                    .map(|c| c.id)?;
                let metrics = core
                    .run(exmo_client.get_user_balances(Nonce::generate()).map_err(ectx!(convert => nonce)))
                    .map(From::from);
                match metrics {
                    Ok(metrics) => {
                        return Ok(metrics);
                    }
                    Err(err) => {
                        thread::sleep(Duration::milliseconds(200).to_std().unwrap());
                        e = err;
                    }
                }
            }
            Err(e)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::*;
    use config::CurrenciesLimits;
    use models::{BTC_DECIMALS, ETH_DECIMALS, STQ_DECIMALS};
    use repos::*;
    use tokio_core::reactor::Core;

    fn create_sell_service(token: AuthenticationToken, user_id: UserId) -> ExchangeServiceImpl<DbExecutorMock> {
        create_sell_service_with_exmo_client(token, user_id, Arc::new(ExmoClientMock::default()))
    }

    fn create_sell_service_with_exmo_client(
        token: AuthenticationToken,
        user_id: UserId,
        exmo_client: Arc<ExmoClientMock>,
    ) -> ExchangeServiceImpl<DbExecutorMock> {
        let auth_service = Arc::new(AuthServiceMock::new(vec![(token, user_id)]));
        let exchange_repo = Arc::new(ExchangesRepoMock::default());
        let sell_orders_repo = Arc::new(SellOrdersRepoMock::default());
        let db_executor = DbExecutorMock::default();
        let reserved_for = 600;
        let rate_upside = 0.1f64;
        let safety_threshold = 0.05f64;
        let limits = CurrenciesLimits::default();
        ExchangeServiceImpl::new(
            auth_service,
            exchange_repo,
            sell_orders_repo,
            db_executor,
            exmo_client,
            reserved_for,
            rate_upside,
            safety_threshold,
            limits,
        )
    }

    #[test]
    #[ignore] // crashes with error 'cannot recursively call into `Core`', need to fix `check_balance`
    fn test_exchange_sell() {
        let token = AuthenticationToken::default();
        let user_id = UserId::generate();
        let service = create_sell_service(token.clone(), user_id);
        let get_rate = GetRate::default();

        let fut = service.get_rate(token.clone(), get_rate).and_then(
            |Exchange {
                 id,
                 from_,
                 to_,
                 amount,
                 amount_currency,
                 ..
             }| {
                let create_sell_order = CreateSellOrder {
                    id,
                    from: from_,
                    to: to_,
                    actual_amount: amount,
                    amount_currency,
                };
                service.sell(token, create_sell_order)
            },
        );

        let mut core = Core::new().unwrap();
        core.run(fut).unwrap();
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
    fn test_thresholds_on_refresh() {
        let token = AuthenticationToken::default();
        let user_id = UserId::generate();
        let exmo_client = Arc::new(ExmoClientMock::default());
        let service = create_sell_service_with_exmo_client(token.clone(), user_id, exmo_client.clone());

        let (pair, order_type) = get_exmo_type(Currency::Stq, Currency::Btc, Currency::Stq)
            .into_iter()
            .next()
            .expect("expected only 1 conversion");
        let amount_super_units = 1_000.0;
        let amount = Currency::Stq.from_f64(amount_super_units);

        let fut = exmo_client
            .clone()
            .get_book(pair.clone())
            .then(|res| {
                let book = res.expect(&format!("failed to get {} order book", pair.clone()));
                book.get_rate(amount_super_units, order_type)
            })
            .then({
                let token = token.clone();
                let service = service.clone();
                move |res| {
                    let original_rate = res.expect("failed to get rate from the order book");
                    let get_rate = GetRate {
                        id: ExchangeId::generate(),
                        from: Currency::Stq,
                        to: Currency::Btc,
                        amount,
                        amount_currency: Currency::Stq,
                    };
                    service.get_rate(token, get_rate).map(move |exchange| (original_rate, exchange))
                }
            })
            .then({
                let token = token.clone();
                let service = service.clone();
                move |res| {
                    let (original_rate, exchange) = res.expect("failed to get rate through the exchange service");
                    let rate_with_upside = exchange.rate;
                    let upside_fraction = rate_with_upside / original_rate;
                    assert!(upside_fraction > 0.89 && upside_fraction < 0.91); // upside threshold in mock = 0.1 (10%)
                    service
                        .refresh_rate(token, exchange.id)
                        .map(move |refreshed_exchange| (original_rate, refreshed_exchange))
                }
            })
            .then({
                let exmo_client = exmo_client.clone();
                let token = token.clone();
                let pair = pair.clone();
                let service = service.clone();
                move |res| {
                    let (original_rate, refreshed_exchange) = res.expect("failed to refresh the rate");
                    assert!(!refreshed_exchange.is_new_rate);

                    // rate worsened by 3% - we still gain 7% which is above the safety threshold in mock (5%)
                    // rate for the user remains the same
                    let worse_rate_within_safety = original_rate * 0.97;
                    exmo_client.set_fixed_rate(pair, worse_rate_within_safety);

                    service
                        .refresh_rate(token, refreshed_exchange.exchange.id)
                        .map(move |refreshed_exchange| (original_rate, refreshed_exchange))
                }
            })
            .then({
                let pair = pair.clone();
                let token = token.clone();
                let service = service.clone();
                move |res| {
                    let (original_rate, refreshed_exchange) = res.expect("failed to refresh the rate");
                    // exchange rate for the user must be still the same because aren't going below the safety threshold
                    assert!(!refreshed_exchange.is_new_rate);

                    // rate worsened by 8% - we only gain 2% which is below the safety threshold in mock (5%)
                    // so we have to update the rate for the user
                    let worse_rate_below_safety = original_rate * 0.92;
                    exmo_client.set_fixed_rate(pair, worse_rate_below_safety);

                    service
                        .refresh_rate(token, refreshed_exchange.exchange.id)
                        .map(move |refreshed_exchange| (worse_rate_below_safety, refreshed_exchange))
                }
            })
            .then(move |res| {
                let (worsened_rate_from_book, ExchangeRefresh { exchange, is_new_rate }) = res.expect("failed to refresh the rate");
                let worsened_rate_with_upside = exchange.rate;
                let new_upside_fraction = worsened_rate_with_upside / worsened_rate_from_book;
                assert!(is_new_rate);
                assert!(new_upside_fraction > 0.89 && new_upside_fraction < 0.91);
                future::ok::<(), ()>(())
            });

        Core::new().unwrap().run(fut).unwrap()
    }

    #[test]
    fn test_rates() {
        let mut core = Core::new().unwrap();
        let token = AuthenticationToken::default();
        let user_id = UserId::generate();
        let service = create_sell_service(token.clone(), user_id);

        let rate = core.run(service.get_current_rate(Currency::Eth, Currency::Btc, Amount::new(ETH_DECIMALS), Currency::Eth));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Btc, Currency::Eth, Amount::new(BTC_DECIMALS), Currency::Eth));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Stq, Currency::Btc, Amount::new(STQ_DECIMALS), Currency::Eth));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Btc, Currency::Stq, Amount::new(BTC_DECIMALS), Currency::Eth));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Stq, Currency::Eth, Amount::new(STQ_DECIMALS), Currency::Eth));
        assert!(rate.is_ok());

        let rate = core.run(service.get_current_rate(Currency::Eth, Currency::Stq, Amount::new(ETH_DECIMALS), Currency::Eth));
        assert!(rate.is_ok());
    }
}
