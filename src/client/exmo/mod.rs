mod error;
mod responses;

pub use self::error::*;

use std::sync::Arc;

use failure::Fail;
use futures::future::{self, Either};
use futures::prelude::*;
use futures::stream::iter_ok;
use hyper::Method;
use hyper::{Body, Request};
use serde::Deserialize;
use serde_json;

use self::responses::*;
use super::HttpClient;
use config::Config;
use models::*;
use utils::read_body;

pub trait ExmoClient: Send + Sync + 'static {
    fn sell(&self, input: ExmoCreateSellOrder) -> Box<Future<Item = ExmoSellOrderResponse, Error = Error> + Send>;
    fn get_current_rate(&self, get: GetRate) -> Box<Future<Item = f64, Error = Error> + Send>;
}

#[derive(Clone)]
pub struct ExmoClientImpl {
    cli: Arc<HttpClient>,
    exmo_url: String,
    api_key: String,
    api_secret: String,
}

impl ExmoClientImpl {
    pub fn new<C: HttpClient>(config: &Config, cli: C) -> Self {
        Self {
            cli: Arc::new(cli),
            exmo_url: config.client.exmo_url.clone(),
            api_key: config.auth.exmo_api_key.clone(),
            api_secret: config.auth.exmo_api_secret.clone(),
        }
    }

    fn exec_query_post<T: for<'de> Deserialize<'de> + Send>(
        &self,
        query: &str,
        body: String,
    ) -> impl Future<Item = T, Error = Error> + Send {
        let query = query.to_string();
        let query1 = query.clone();
        let query2 = query.clone();
        let query3 = query.clone();
        let cli = self.cli.clone();
        let mut builder = Request::builder();
        let url = format!("{}{}", self.exmo_url, query);
        builder
            .uri(url)
            .method(Method::POST)
            .body(Body::from(body))
            .map_err(ectx!(ErrorSource::Hyper, ErrorKind::MalformedInput => query3))
            .into_future()
            .and_then(move |req| cli.request(req).map_err(ectx!(ErrorKind::Internal => query1)))
            .and_then(move |resp| read_body(resp.into_body()).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal => query2)))
            .and_then(|bytes| {
                let bytes_clone = bytes.clone();
                String::from_utf8(bytes).map_err(ectx!(ErrorSource::Utf8, ErrorKind::Internal => bytes_clone))
            }).and_then(|string| serde_json::from_str::<T>(&string).map_err(ectx!(ErrorSource::Json, ErrorKind::Internal => string)))
    }

    fn exec_query_get<T: for<'de> Deserialize<'de> + Send>(&self, query: &str) -> impl Future<Item = T, Error = Error> + Send {
        let query = query.to_string();
        let query1 = query.clone();
        let query2 = query.clone();
        let cli = self.cli.clone();
        let url = format!("{}{}", self.exmo_url, query);
        cli.get(url)
            .map_err(ectx!(ErrorKind::Internal => query1))
            .and_then(move |resp| read_body(resp.into_body()).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal => query2)))
            .and_then(|bytes| {
                let bytes_clone = bytes.clone();
                String::from_utf8(bytes).map_err(ectx!(ErrorSource::Utf8, ErrorKind::Internal => bytes_clone))
            }).and_then(|string| serde_json::from_str::<T>(&string).map_err(ectx!(ErrorSource::Json, ErrorKind::Internal => string)))
    }

    fn get_book(&self, currency: Currency) -> Box<Future<Item = ExmoBook, Error = Error> + Send> {
        let pair = match currency {
            Currency::Eth => "ETH_BTC",
            Currency::Stq => "STQ_BTC",
            _ => {
                return Box::new(future::err(
                    ectx!(err ErrorSource::NoSuchConversion, ErrorKind::Internal => "BTC_BTC"),
                ))
            }
        };
        let url = format!("/order_book/?pair={}", pair);
        Box::new(self.exec_query_get::<ExmoRateResponse>(&url).and_then(move |resp| {
            if let Some(book) = resp.pair.get(pair) {
                Either::A(future::ok(book.clone()))
            } else {
                Either::B(future::err(ectx!(err ErrorSource::Json, ErrorKind::Internal => resp)))
            }
        }))
    }

    fn get_rate(&self, currencies_exchange: Vec<(Currency, OrderType)>, amount: Amount) -> Box<Future<Item = f64, Error = Error> + Send> {
        let service = self.clone();
        Box::new(iter_ok::<_, Error>(currencies_exchange).fold(1f64, move |rate, currency_exchange| {
            let (currency, order_type) = currency_exchange;
            service
                .get_book(currency)
                .and_then(move |book| book.get_rate(currency.to_f64(amount), order_type))
                .and_then(move |mut res| {
                    if need_revert(order_type) {
                        res = 1f64 / res;
                    };
                    Ok(rate * res) as Result<f64, Error>
                })
        }))
    }
}

impl ExmoClient for ExmoClientImpl {
    fn sell(&self, _input: ExmoCreateSellOrder) -> Box<Future<Item = ExmoSellOrderResponse, Error = Error> + Send> {
        unimplemented!()
    }
    fn get_current_rate(&self, get: GetRate) -> Box<Future<Item = f64, Error = Error> + Send> {
        let service = self.clone();
        let amount = get.amount;
        Box::new(
            get_exmo_type(get.from, get.to)
                .into_future()
                .and_then(move |currencies_exchange| service.get_rate(currencies_exchange, amount)),
        )
    }
}

/// All exchanges are done for prices in BTC, therefore
/// we need to set what we need to do - sell, or buy
fn get_exmo_type(from: Currency, to: Currency) -> Result<Vec<(Currency, OrderType)>, Error> {
    match (from, to) {
        (Currency::Btc, Currency::Eth) => Ok(vec![(Currency::Eth, OrderType::Sell)]),
        (Currency::Eth, Currency::Btc) => Ok(vec![(Currency::Eth, OrderType::Buy)]),
        (Currency::Btc, Currency::Stq) => Ok(vec![(Currency::Stq, OrderType::Sell)]),
        (Currency::Stq, Currency::Btc) => Ok(vec![(Currency::Stq, OrderType::Buy)]),
        (Currency::Eth, Currency::Stq) => Ok(vec![(Currency::Eth, OrderType::Buy), (Currency::Stq, OrderType::Sell)]),
        (Currency::Stq, Currency::Eth) => Ok(vec![(Currency::Stq, OrderType::Buy), (Currency::Eth, OrderType::Sell)]),
        (_, _) => Err(ectx!(err ErrorSource::NoSuchConversion, ErrorKind::Internal => from, to)),
    }
}

/// All exchanges are done for prices in BTC, therefore
/// if we are buying ETH or STQ we get rate for BTC
/// and it does not need to be reverted. Opposite,
/// if we are selling ETH or STQ we need to revert it
fn need_revert(order_type: OrderType) -> bool {
    match order_type {
        OrderType::Buy => false,
        OrderType::Sell => true,
    }
}

#[derive(Default)]
pub struct ExmoClientMock;

impl ExmoClient for ExmoClientMock {
    fn sell(&self, _input: ExmoCreateSellOrder) -> Box<Future<Item = ExmoSellOrderResponse, Error = Error> + Send> {
        Box::new(Ok(ExmoSellOrderResponse::default()).into_future())
    }
    fn get_current_rate(&self, _get: GetRate) -> Box<Future<Item = f64, Error = Error> + Send> {
        Box::new(Ok(1f64).into_future())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::HttpClientImpl;
    use config;
    use models::{BTC_DECIMALS, ETH_DECIMALS, STQ_DECIMALS};
    use tokio_core::reactor::Core;

    fn create_client() -> ExmoClientImpl {
        let config = config::Config::new().unwrap_or_else(|e| panic!("Error parsing config: {}", e));
        let client = HttpClientImpl::new(&config);
        ExmoClientImpl::new(&config, client)
    }

    #[test]
    fn test_rates() {
        let client = create_client();
        let mut core = Core::new().unwrap();
        let mut get_rate = GetRate::default();
        get_rate.from = Currency::Eth;
        get_rate.to = Currency::Btc;
        get_rate.amount = Amount::new(ETH_DECIMALS);
        let rate = core.run(client.get_current_rate(get_rate.clone())).unwrap();
        println!("from {} to {} with {}, rate = {}", get_rate.from, get_rate.to, get_rate.amount, rate);
        // assert!(rate < 1f64);

        get_rate.from = Currency::Btc;
        get_rate.to = Currency::Eth;
        get_rate.amount = Amount::new(BTC_DECIMALS);
        let rate = core.run(client.get_current_rate(get_rate.clone())).unwrap();
        println!("from {} to {} with {}, rate = {}", get_rate.from, get_rate.to, get_rate.amount, rate);
        // assert!(rate > 1f64);

        get_rate.from = Currency::Stq;
        get_rate.to = Currency::Btc;
        get_rate.amount = Amount::new(STQ_DECIMALS);
        let rate = core.run(client.get_current_rate(get_rate.clone())).unwrap();
        println!("from {} to {} with {}, rate = {}", get_rate.from, get_rate.to, get_rate.amount, rate);
        // assert!(rate < 1f64);

        get_rate.from = Currency::Btc;
        get_rate.to = Currency::Stq;
        get_rate.amount = Amount::new(STQ_DECIMALS);
        let rate = core.run(client.get_current_rate(get_rate.clone())).unwrap();
        println!("from {} to {} with {}, rate = {}", get_rate.from, get_rate.to, get_rate.amount, rate);
        // assert!(rate > 1f64);

        get_rate.from = Currency::Stq;
        get_rate.to = Currency::Eth;
        get_rate.amount = Amount::new(STQ_DECIMALS);
        let rate = core.run(client.get_current_rate(get_rate.clone())).unwrap();
        println!("from {} to {} with {}, rate = {}", get_rate.from, get_rate.to, get_rate.amount, rate);
        // assert!(rate < 1f64);

        get_rate.from = Currency::Eth;
        get_rate.to = Currency::Stq;
        get_rate.amount = Amount::new(ETH_DECIMALS);
        let rate = core.run(client.get_current_rate(get_rate.clone())).unwrap();
        println!("from {} to {} with {}, rate = {}", get_rate.from, get_rate.to, get_rate.amount, rate);
        // assert!(rate > 1f64);
    }
}
