use failure::Fail;
use futures::future;
use futures::prelude::*;
use hyper;
use sentry::integrations::failure::capture_error;

use models::*;

pub fn format_error<E: Fail>(error: &E) -> String {
    let mut result = String::new();
    let mut chain: Vec<&Fail> = Vec::new();
    let mut iter: Option<&Fail> = Some(error);
    while let Some(e) = iter {
        chain.push(e);
        iter = e.cause();
    }
    for err in chain.into_iter().rev() {
        result.push_str(&format!("{}\n", err));
    }
    if let Some(bt) = error.backtrace() {
        let bt = format!("{}", bt);
        let lines: Vec<&str> = bt.split('\n').skip(1).collect();
        if lines.is_empty() {
            result.push_str("\nRelevant backtrace:\n");
        }
        lines.chunks(2).for_each(|chunk| {
            if let Some(line1) = chunk.get(0) {
                if line1.contains("transactions_lib") {
                    result.push_str(line1);
                    result.push_str("\n");
                    if let Some(line2) = chunk.get(1) {
                        result.push_str(line2);
                        result.push_str("\n");
                    }
                }
            }
        });
    }
    result
}

pub fn log_error<E: Fail>(error: &E) {
    error!("\n{}", format_error(error));
}

pub fn log_and_capture_error<E: Fail>(error: E) {
    log_error(&error);
    capture_error(&error.into());
}

pub fn log_warn<E: Fail>(error: &E) {
    warn!("\n{}", format_error(error));
}

// Reads body of request in Future format
pub fn read_body(body: hyper::Body) -> impl Future<Item = Vec<u8>, Error = hyper::Error> {
    body.fold(Vec::new(), |mut acc, chunk| {
        acc.extend_from_slice(&*chunk);
        future::ok::<_, hyper::Error>(acc)
    })
}

/// All exchanges are done for prices in BTC, therefore
/// we need to set what we need to do - sell, or buy
pub fn get_exmo_type(from: Currency, to: Currency) -> Vec<(String, OrderType)> {
    match (from, to) {
        (Currency::Btc, Currency::Eth) => vec![("ETH_BTC".to_string(), OrderType::Sell)],
        (Currency::Eth, Currency::Btc) => vec![("ETH_BTC".to_string(), OrderType::Buy)],
        (Currency::Btc, Currency::Stq) => vec![("STQ_BTC".to_string(), OrderType::Sell)],
        (Currency::Stq, Currency::Btc) => vec![("STQ_BTC".to_string(), OrderType::Buy)],
        (Currency::Eth, Currency::Stq) => vec![("ETH_BTC".to_string(), OrderType::Buy), ("STQ_BTC".to_string(), OrderType::Sell)],
        (Currency::Stq, Currency::Eth) => vec![("STQ_BTC".to_string(), OrderType::Buy), ("ETH_BTC".to_string(), OrderType::Sell)],
        (Currency::Stq, Currency::Stq) => vec![("STQ_STQ".to_string(), OrderType::Buy)],
        (Currency::Btc, Currency::Btc) => vec![("BTC_BTC".to_string(), OrderType::Buy)],
        (Currency::Eth, Currency::Eth) => vec![("ETH_ETH".to_string(), OrderType::Buy)],
    }
}

/// All exchanges are done for prices in BTC, therefore
/// if we are buying ETH or STQ we get rate for BTC
/// and it does not need to be reverted. Opposite,
/// if we are selling ETH or STQ we need to revert it
pub fn need_revert(order_type: OrderType) -> bool {
    match order_type {
        OrderType::Buy => false,
        OrderType::Sell => true,
    }
}
