mod error;
mod responses;

pub use self::error::*;
use self::responses::*;
use super::HttpClient;
use config::Config;
use failure::Fail;
use futures::prelude::*;
use hyper::Method;
use hyper::{Body, Request};
use models::*;
use serde::Deserialize;
use serde_json;
use std::sync::Arc;
use utils::read_body;

pub trait ExmoClient: Send + Sync + 'static {
    fn sell(&self, input: ExmoCreateSellOrder) -> Box<Future<Item = ExmoSellOrderResponse, Error = Error> + Send>;
    fn get_rate(&self, get: GetRate) -> Box<Future<Item = ExmoGetExchangeResponse, Error = Error> + Send>;
}

pub struct ExmoClientImpl {
    cli: Arc<HttpClient>,
    exmo_url: String,
    rate_upside: f64,
    safety_threshold: f64,
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
            rate_upside: config.exchange_options.rate_upside,
            safety_threshold: config.exchange_options.safety_threshold,
        }
    }

    fn exec_query<T: for<'de> Deserialize<'de> + Send>(&self, query: &str) -> impl Future<Item = T, Error = Error> + Send {
        let query = query.to_string();
        let query1 = query.clone();
        let query2 = query.clone();
        let query3 = query.clone();
        let cli = self.cli.clone();
        let query = query.replace("\n", "");
        let body = format!(
            r#"
                {{
                    "operationName": "M",
                    "query": "{}",
                    "variables": null
                }}
            "#,
            query
        );
        let mut builder = Request::builder();
        builder.uri(self.exmo_url.clone()).method(Method::POST);
        let token = self.api_key.clone();
        builder.header("Authorization", format!("Bearer {}", token));
        builder
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
}

impl ExmoClient for ExmoClientImpl {
    fn sell(&self, input: ExmoCreateSellOrder) -> Box<Future<Item = ExmoSellOrderResponse, Error = Error> + Send> {
        let rate_upside = self.rate_upside;
        let safety_threshold = self.safety_threshold;
        unimplemented!()
    }
    fn get_rate(&self, get: GetRate) -> Box<Future<Item = ExmoGetExchangeResponse, Error = Error> + Send> {
        let rate_upside = self.rate_upside;
        let safety_threshold = self.safety_threshold;
        unimplemented!()
    }
    // fn confirm_reset_password(&self, reset: ResetPasswordConfirm) -> Box<Future<Item = ExmoJWT, Error = Error> + Send> {
    //     let query = format!(
    //         r#"
    //             mutation M {{
    //                 applyPasswordReset(input: {{token: \"{}\", password: \"{}\", clientMutationId:\"\"}}) {{
    //                     success
    //                     token
    //                 }}
    //             }}
    //         "#,
    //         reset.token, reset.password,
    //     );
    //     Box::new(
    //         self.exec_query::<ResetApply>(&query, None)
    //             .and_then(|resp| {
    //                 resp.data
    //                     .clone()
    //                     .ok_or(ectx!(err ErrorContext::NoGraphQLData, ErrorKind::Unauthorized => resp))
    //             }).map(|resp_data| resp_data.token),
    //     )
    // }
}

#[derive(Default)]
pub struct ExmoClientMock;

impl ExmoClient for ExmoClientMock {
    fn sell(&self, _input: ExmoCreateSellOrder) -> Box<Future<Item = ExmoSellOrderResponse, Error = Error> + Send> {
        Box::new(Ok(ExmoSellOrderResponse::default()).into_future())
    }
    fn get_rate(&self, _get: GetRate) -> Box<Future<Item = ExmoGetExchangeResponse, Error = Error> + Send> {
        Box::new(Ok(ExmoGetExchangeResponse::default()).into_future())
    }
}
