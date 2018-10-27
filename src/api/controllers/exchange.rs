use failure::Fail;
use futures::prelude::*;

use super::super::utils::{parse_body, response_with_model};
use super::Context;
use super::ControllerFuture;
use api::error::*;
use api::requests::*;
use api::responses::*;
use serde_qs;

pub fn get_exchange(ctx: &Context) -> ControllerFuture {
    let exchange_service = ctx.exchange_service.clone();
    let path_and_query = ctx.uri.path_and_query();
    let path_and_query_clone = ctx.uri.path_and_query();
    Box::new(
        ctx.uri
            .query()
            .ok_or(ectx!(err ErrorContext::RequestMissingQuery, ErrorKind::BadRequest => path_and_query))
            .and_then(|query| {
                serde_qs::from_str::<GetExchangeParams>(query).map_err(|e| {
                    let e = format_err!("{}", e);
                    ectx!(err e, ErrorContext::RequestQueryParams, ErrorKind::BadRequest => path_and_query_clone)
                })
            }).into_future()
            .and_then(move |input| {
                let input_clone = input.clone();
                exchange_service.get_exchange(input.into()).map_err(ectx!(convert => input_clone))
            }).and_then(|exchange| response_with_model(&ExchangeResponse::from(exchange))),
    )
}

pub fn post_exchange(ctx: &Context) -> ControllerFuture {
    let exchange_service = ctx.exchange_service.clone();
    let body = ctx.body.clone();
    Box::new(
        parse_body::<PostExchangeRequest>(body)
            .and_then(move |input| {
                let input_clone = input.clone();
                exchange_service.sell(input.into()).map_err(ectx!(convert => input_clone))
            }).and_then(|sell| response_with_model(&SellOrderResponse::from(sell))),
    )
}
