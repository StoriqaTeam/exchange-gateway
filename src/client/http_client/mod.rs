mod error;

use std::time::Duration;

use config::Config;
use failure::Fail;
use futures::future::Either;
use futures::prelude::*;
use hyper;
use hyper::{client::HttpConnector, Body, Request, Response};
use hyper_tls::HttpsConnector;
use log::{self, Level};
use tokio_timer::Timeout;

use self::error::*;
use utils::read_body;

const TIMEOUT_SEC: u64 = 5;

pub trait HttpClient: Send + Sync + 'static {
    fn request(&self, req: Request<Body>) -> Box<Future<Item = Response<Body>, Error = Error> + Send>;
    fn get(&self, uri: String) -> Box<Future<Item = Response<Body>, Error = Error> + Send>;
}

#[derive(Clone)]
pub struct HttpClientImpl {
    cli: hyper::Client<HttpsConnector<HttpConnector>>,
    timeout_s: u64,
}

impl HttpClientImpl {
    pub fn new(config: &Config) -> Self {
        let connector = HttpsConnector::new(config.client.dns_threads).unwrap();
        let cli = hyper::Client::builder().build(connector);
        let timeout_s = config.client.timeout_s.unwrap_or(TIMEOUT_SEC);
        Self { cli, timeout_s }
    }
}

impl HttpClient for HttpClientImpl {
    fn request(&self, req: Request<Body>) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {
        let cli = self.cli.clone();
        let timeout_s = Duration::from_secs(self.timeout_s);
        let level = log::max_level();
        let fut = if level == Level::Debug || level == Level::Trace {
            let (parts, body) = req.into_parts();
            Either::A(
                read_body(body)
                    .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                    .and_then(move |body| {
                        debug!(
                            "HttpClient, sent request {} {}, headers: {:#?}, body: {:?}",
                            parts.method,
                            parts.uri,
                            parts.headers,
                            String::from_utf8(body.clone()).ok()
                        );
                        let req = Request::from_parts(parts, body.into());
                        Timeout::new(cli.request(req), timeout_s).map_err(|e| {
                            if e.is_inner() {
                                ectx!(err e, ErrorSource::Hyper, ErrorKind::Internal)
                            } else {
                                ectx!(err ErrorSource::Hyper, ErrorKind::GatewayTimeout)
                            }
                        })
                    })
                    .and_then(|resp| {
                        let (parts, body) = resp.into_parts();
                        read_body(body)
                            .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                            .map(|body| (parts, body))
                    })
                    .map(|(parts, body)| {
                        debug!(
                            "HttpClient, recieved response with status {} headers: {:#?} and body: {:?}",
                            parts.status.as_u16(),
                            parts.headers,
                            String::from_utf8(body.clone()).ok()
                        );
                        Response::from_parts(parts, body.into())
                    }),
            )
        } else {
            Either::B(cli.request(req).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal)))
        };

        Box::new(fut.and_then(|resp| {
            if resp.status().is_client_error() || resp.status().is_server_error() {
                match resp.status().as_u16() {
                    400 => Err(ectx!(err ErrorSource::Server, ErrorKind::BadRequest)),
                    401 => Err(ectx!(err ErrorSource::Server, ErrorKind::Unauthorized)),
                    404 => Err(ectx!(err ErrorSource::Server, ErrorKind::NotFound)),
                    500 => Err(ectx!(err ErrorSource::Server, ErrorKind::Internal)),
                    502 => Err(ectx!(err ErrorSource::Server, ErrorKind::BadGateway)),
                    504 => Err(ectx!(err ErrorSource::Server, ErrorKind::GatewayTimeout)),
                    _ => Err(ectx!(err ErrorSource::Server, ErrorKind::UnknownServerError)),
                }
            } else {
                Ok(resp)
            }
        }))
    }
    fn get(&self, uri: String) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {
        let cli = self.cli.clone();
        let timeout_s = Duration::from_secs(self.timeout_s);
        let level = log::max_level();
        let fut = if level == Level::Debug || level == Level::Trace {
            Either::A(
                uri.clone()
                    .parse()
                    .map_err(|_| ectx!(err ErrorSource::Hyper, ErrorKind::Internal => uri))
                    .into_future()
                    .and_then(move |uri| {
                        debug!("HttpClient, sent request GET {}", uri);
                        Timeout::new(cli.get(uri), timeout_s).map_err(|e| {
                            if e.is_inner() {
                                ectx!(err e, ErrorSource::Hyper, ErrorKind::Internal)
                            } else {
                                ectx!(err ErrorSource::Hyper, ErrorKind::GatewayTimeout)
                            }
                        })
                    })
                    .and_then(|resp| {
                        let (parts, body) = resp.into_parts();
                        read_body(body)
                            .map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))
                            .map(|body| (parts, body))
                    })
                    .map(|(parts, body)| {
                        debug!(
                            "HttpClient, recieved response with status {} headers: {:#?} and body: {:?}",
                            parts.status.as_u16(),
                            parts.headers,
                            String::from_utf8(body.clone()).ok()
                        );
                        Response::from_parts(parts, body.into())
                    }),
            )
        } else {
            Either::B(
                uri.clone()
                    .parse()
                    .map_err(|_| ectx!(err ErrorSource::Hyper, ErrorKind::Internal => uri))
                    .into_future()
                    .and_then(move |uri| cli.get(uri).map_err(ectx!(ErrorSource::Hyper, ErrorKind::Internal))),
            )
        };

        Box::new(fut.and_then(|resp| {
            if resp.status().is_client_error() || resp.status().is_server_error() {
                match resp.status().as_u16() {
                    400 => Err(ectx!(err ErrorSource::Server, ErrorKind::BadRequest)),
                    401 => Err(ectx!(err ErrorSource::Server, ErrorKind::Unauthorized)),
                    404 => Err(ectx!(err ErrorSource::Server, ErrorKind::NotFound)),
                    500 => Err(ectx!(err ErrorSource::Server, ErrorKind::Internal)),
                    502 => Err(ectx!(err ErrorSource::Server, ErrorKind::BadGateway)),
                    504 => Err(ectx!(err ErrorSource::Server, ErrorKind::GatewayTimeout)),
                    _ => Err(ectx!(err ErrorSource::Server, ErrorKind::UnknownServerError)),
                }
            } else {
                Ok(resp)
            }
        }))
    }
}
