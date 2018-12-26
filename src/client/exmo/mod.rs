mod error;
mod responses;
mod seeds;

pub use self::error::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha2::Sha512;
use failure::Fail;
use futures::future::{self, Either};
use futures::prelude::*;
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
    fn create_order(
        &self,
        pair: String,
        quantity: f64,
        order_type: OrderType,
        nonce: Nonce,
    ) -> Box<Future<Item = u64, Error = Error> + Send>;
    fn get_order_status(&self, order_id: u64, nonce: Nonce) -> Box<Future<Item = (f64, f64), Error = Error> + Send>;
    fn get_book(&self, pair: String) -> Box<Future<Item = ExmoBook, Error = Error> + Send>;
    fn get_user_trades(&self, pair: String, nonce: Nonce) -> Box<Future<Item = (f64, f64), Error = Error> + Send>;
    fn get_user_balances(&self, nonce: Nonce) -> Box<Future<Item = ExmoUserInfo, Error = Error> + Send>;
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

    fn sign(&self, message: String) -> String {
        let key_bytes = self.api_secret.as_bytes();
        let mut hmac = Hmac::new(Sha512::new(), key_bytes);
        let message_bytes = message.as_bytes();
        hmac.input(message_bytes);
        let mut output = [0; 64];
        hmac.raw_result(&mut output);
        output.into_iter().fold("".to_string(), |mut acc, x| {
            acc += &format!("{:02x}", x);
            acc
        })
    }

    fn exec_query_post<T: for<'de> Deserialize<'de> + Send>(
        &self,
        query: &str,
        message: String,
    ) -> impl Future<Item = T, Error = Error> + Send {
        let query = query.to_string();
        let query1 = query.clone();
        let query2 = query.clone();
        let query3 = query.clone();
        let cli = self.cli.clone();
        let mut builder = Request::builder();
        let url = format!("{}{}", self.exmo_url, query);
        let key = self.api_key.clone();
        let sign = self.sign(message.clone());
        builder
            .uri(url)
            .method(Method::POST)
            .header("Key", key)
            .header("Sign", sign)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::from(message))
            .map_err(ectx!(ErrorSource::Hyper, ErrorKind::MalformedInput => query3))
            .into_future()
            .and_then(move |req| cli.request(req).map_err(ectx!(ErrorKind::Internal => query1)))
            .and_then(move |resp| read_body(resp.into_body()).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal => query2)))
            .and_then(|bytes| {
                let bytes_clone = bytes.clone();
                String::from_utf8(bytes).map_err(ectx!(ErrorSource::Utf8, ErrorKind::Internal => bytes_clone))
            })
            .and_then(|string| serde_json::from_str::<T>(&string).map_err(ectx!(ErrorSource::Json, ErrorKind::Internal => string)))
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
            })
            .and_then(|string| serde_json::from_str::<T>(&string).map_err(ectx!(ErrorSource::Json, ErrorKind::Internal => string)))
    }
}

impl ExmoClient for ExmoClientImpl {
    fn create_order(
        &self,
        pair: String,
        quantity: f64,
        order_type: OrderType,
        nonce: Nonce,
    ) -> Box<Future<Item = u64, Error = Error> + Send> {
        let message = format!("nonce={}&pair={}&quantity={}&price=0&type={}", nonce, pair, quantity, order_type);
        let url = format!("/order_create");
        Box::new(
            self.exec_query_post::<ExmoCreateOrderResponse>(&url, message)
                .and_then(move |resp| {
                    if resp.result {
                        Either::A(future::ok(resp.order_id))
                    } else {
                        Either::B(future::err(ectx!(err ErrorSource::Json, ErrorKind::Internal => resp.error)))
                    }
                }),
        )
    }

    fn get_order_status(&self, order_id: u64, nonce: Nonce) -> Box<Future<Item = (f64, f64), Error = Error> + Send> {
        let message = format!("nonce={}&order_id={}", nonce, order_id);
        let url = format!("/order_trades");
        Box::new(
            self.exec_query_post::<ExmoOrderResponse>(&url, message)
                .map(|resp| (resp.in_amount, resp.out_amount)),
        )
    }

    fn get_book(&self, pair: String) -> Box<Future<Item = ExmoBook, Error = Error> + Send> {
        let url = format!("/order_book/?pair={}", pair);
        Box::new(self.exec_query_get::<ExmoRateResponse>(&url).and_then(move |resp| {
            if let Some(book) = resp.pair.get(&pair) {
                Either::A(future::ok(book.clone()))
            } else {
                Either::B(future::err(ectx!(err ErrorSource::Json, ErrorKind::Internal => resp)))
            }
        }))
    }

    fn get_user_trades(&self, pair: String, nonce: Nonce) -> Box<Future<Item = (f64, f64), Error = Error> + Send> {
        let message = format!("nonce={}&pair={}", nonce, pair);
        let url = format!("/user_trades");
        Box::new(
            self.exec_query_post::<ExmoOrderResponse>(&url, message)
                .map(|resp| (resp.in_amount, resp.out_amount)),
        )
    }
    fn get_user_balances(&self, nonce: Nonce) -> Box<Future<Item = ExmoUserInfo, Error = Error> + Send> {
        let message = format!("nonce={}", nonce);
        let url = format!("/user_info");
        Box::new(self.exec_query_post::<ExmoUserInfo>(&url, message))
    }
}

pub struct ExmoClientMock {
    data: Arc<Mutex<HashMap<String, ExmoBook>>>,
    orders: Arc<Mutex<Vec<ExmoOrder>>>,
}

impl ExmoClientMock {
    #[allow(dead_code)]
    pub fn set_fixed_rate(&self, pair: String, rate: f64) {
        let new_book = seeds::make_book_for_rate(rate);

        let mut data = self.data.lock().unwrap();
        (*data).insert(pair, new_book);
    }
}

impl Default for ExmoClientMock {
    fn default() -> Self {
        let orders = Arc::new(Mutex::new(vec![]));
        let stq_btc_book = seeds::get_stq_btc();
        let eth_btc_book = seeds::get_eth_btc();
        let stq_usd_book = seeds::get_stq_usd();
        let eth_usd_book = seeds::get_eth_usd();
        let mut data_hash = HashMap::new();
        data_hash.insert("STQ_BTC".to_string(), stq_btc_book);
        data_hash.insert("ETH_BTC".to_string(), eth_btc_book);
        data_hash.insert("STQ_USD".to_string(), stq_usd_book);
        data_hash.insert("ETH_USD".to_string(), eth_usd_book);
        let data = Arc::new(Mutex::new(data_hash));
        Self { data, orders }
    }
}

impl ExmoClient for ExmoClientMock {
    fn create_order(
        &self,
        pair: String,
        quantity: f64,
        order_type: OrderType,
        _nonce: Nonce,
    ) -> Box<Future<Item = u64, Error = Error> + Send> {
        let mut orders = self.orders.lock().unwrap();
        let data = self.data.lock().unwrap();

        let book = if let Some(book) = data.get(&pair) {
            book.clone()
        } else {
            ExmoBook::default()
        };
        let price = book.get_rate(quantity, order_type).unwrap_or_default();
        let amount = price * quantity;
        let order = ExmoOrder::new(price, quantity, amount);
        orders.push(order);
        Box::new(Ok((orders.len() - 1) as u64).into_future())
    }
    fn get_order_status(&self, order_id: u64, _nonce: Nonce) -> Box<Future<Item = (f64, f64), Error = Error> + Send> {
        let orders = self.orders.lock().unwrap();
        let (in_amount, out_amount) = if let Some(order) = orders.iter().nth(order_id as usize) {
            (order.quantity, order.quantity * 2f64)
        } else {
            (1f64, 1f64)
        };
        Box::new(Ok((in_amount, out_amount)).into_future())
    }
    fn get_book(&self, pair: String) -> Box<Future<Item = ExmoBook, Error = Error> + Send> {
        let data = self.data.lock().unwrap();
        let book = if let Some(book) = data.get(&pair) {
            book.clone()
        } else {
            ExmoBook::default()
        };
        Box::new(Ok(book).into_future())
    }
    fn get_user_trades(&self, _pair: String, _nonce: Nonce) -> Box<Future<Item = (f64, f64), Error = Error> + Send> {
        Box::new(Ok((1f64, 1f64)).into_future())
    }
    fn get_user_balances(&self, _nonce: Nonce) -> Box<Future<Item = ExmoUserInfo, Error = Error> + Send> {
        Box::new(Ok(ExmoUserInfo::default()).into_future())
    }
}

#[cfg(test)]
mod tests {
    use futures::stream::iter_ok;
    use tokio_core::reactor::Core;

    use super::*;
    use client::HttpClientImpl;
    use config;
    use models::{BTC_DECIMALS, ETH_DECIMALS, STQ_DECIMALS};
    use utils::{get_exmo_type, need_revert};

    fn get_rate(
        service: &ExmoClientImpl,
        currencies_exchange: Vec<(String, OrderType)>,
        amount: f64,
    ) -> Box<Future<Item = f64, Error = Error> + Send> {
        let service = service.clone();
        Box::new(iter_ok::<_, Error>(currencies_exchange).fold(1f64, move |rate, currency_exchange| {
            let (pair, order_type) = currency_exchange;
            service
                .get_book(pair)
                .and_then(move |book| book.get_rate(amount, order_type))
                .and_then(move |mut res| {
                    if need_revert(order_type) {
                        res = 1f64 / res;
                    };
                    Ok(rate * res) as Result<f64, Error>
                })
        }))
    }

    fn create_client() -> ExmoClientImpl {
        let config = config::Config::new().unwrap_or_else(|e| panic!("Error parsing config: {}", e));
        let client = HttpClientImpl::new(&config);
        ExmoClientImpl::new(&config, client)
    }

    #[test]
    fn test_exmo_get_rates_straight() {
        let client = create_client();
        let mut core = Core::new().unwrap();

        let input = get_exmo_type(Currency::Eth, Currency::Btc, Currency::Eth);
        let amount = Currency::Eth.to_f64(Amount::new(ETH_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate < 1f64);

        let input = get_exmo_type(Currency::Btc, Currency::Eth, Currency::Btc);
        let amount = Currency::Btc.to_f64(Amount::new(BTC_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate > 1f64);

        let input = get_exmo_type(Currency::Stq, Currency::Btc, Currency::Stq);
        let amount = Currency::Stq.to_f64(Amount::new(STQ_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate < 1f64);

        let input = get_exmo_type(Currency::Btc, Currency::Stq, Currency::Btc);
        let amount = Currency::Btc.to_f64(Amount::new(BTC_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate > 1f64);

        let input = get_exmo_type(Currency::Stq, Currency::Eth, Currency::Stq);
        let amount = Currency::Stq.to_f64(Amount::new(STQ_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate < 1f64);

        let input = get_exmo_type(Currency::Eth, Currency::Stq, Currency::Eth);
        let amount = Currency::Eth.to_f64(Amount::new(ETH_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate > 1f64);
    }

    #[test]
    fn test_exmo_get_rates_invert() {
        let client = create_client();
        let mut core = Core::new().unwrap();

        let input = get_exmo_type(Currency::Eth, Currency::Btc, Currency::Btc);
        let amount = Currency::Btc.to_f64(Amount::new(BTC_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate < 1f64);

        let input = get_exmo_type(Currency::Btc, Currency::Eth, Currency::Eth);
        let amount = Currency::Eth.to_f64(Amount::new(ETH_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate > 1f64);

        let input = get_exmo_type(Currency::Stq, Currency::Btc, Currency::Btc);
        let amount = Currency::Btc.to_f64(Amount::new(BTC_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate < 1f64);

        let input = get_exmo_type(Currency::Btc, Currency::Stq, Currency::Stq);
        let amount = Currency::Stq.to_f64(Amount::new(STQ_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate > 1f64);

        let input = get_exmo_type(Currency::Stq, Currency::Eth, Currency::Eth);
        let amount = Currency::Eth.to_f64(Amount::new(ETH_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate < 1f64);

        let input = get_exmo_type(Currency::Eth, Currency::Stq, Currency::Stq);
        let amount = Currency::Stq.to_f64(Amount::new(STQ_DECIMALS));
        let rate = core.run(get_rate(&client, input.clone(), amount)).unwrap();
        assert!(rate > 1f64);
    }
}
