use failure::Fail;
use futures::prelude::*;

use super::super::utils::{parse_body, response_with_model};
use super::Context;
use super::ControllerFuture;
use api::error::*;
use api::requests::*;
use api::responses::*;

pub fn post_rate(ctx: &Context) -> ControllerFuture {
    let exchange_service = ctx.exchange_service.clone();
    let body = ctx.body.clone();
    let maybe_token = ctx.get_auth_token();

    Box::new(
        maybe_token
            .ok_or_else(|| ectx!(err ErrorContext::Token, ErrorKind::Unauthorized))
            .into_future()
            .and_then(move |token| {
                parse_body::<PostRateRequest>(body)
                    .and_then(move |input| {
                        let input_clone = input.clone();
                        exchange_service
                            .get_rate(token, input.into())
                            .map_err(ectx!(convert => input_clone))
                    }).and_then(|exchange| response_with_model(&ExchangeResponse::from(exchange)))
            }),
    )
}

pub fn post_exchange(ctx: &Context) -> ControllerFuture {
    let exchange_service = ctx.exchange_service.clone();
    let maybe_token = ctx.get_auth_token();
    let body = ctx.body.clone();
    Box::new(
        maybe_token
            .ok_or_else(|| ectx!(err ErrorContext::Token, ErrorKind::Unauthorized))
            .into_future()
            .and_then(move |token| {
                parse_body::<PostExchangeRequest>(body)
                    .and_then(move |input| {
                        let input_clone = input.clone();
                        exchange_service.sell(token, input.into()).map_err(ectx!(convert => input_clone))
                    }).and_then(|sell| response_with_model(&SellOrderResponse::from(sell)))
            }),
    )
}

pub fn get_metrics(ctx: &Context) -> ControllerFuture {
    let exchange_service = ctx.exchange_service.clone();
    let maybe_token = ctx.get_auth_token();
    Box::new(
        maybe_token
            .ok_or_else(|| ectx!(err ErrorContext::Token, ErrorKind::Unauthorized))
            .into_future()
            .and_then(move |_| {
                exchange_service
                    .metrics()
                    .map_err(ectx!(convert))
                    .and_then(|metrics| response_with_model(&MetricsResponse::from(metrics)))
            }),
    )
}
