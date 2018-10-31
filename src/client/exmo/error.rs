use std::fmt;
use std::fmt::Display;

use failure::{Backtrace, Context, Fail};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "exmo client error - malformed input")]
    MalformedInput,
    #[fail(display = "exmo client error - unauthorized")]
    Unauthorized,
    #[fail(display = "exmo client error - internal error")]
    Internal,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorSource {
    #[fail(display = "exmo client source - error inside of Hyper library")]
    Hyper,
    #[fail(display = "exmo client source - error parsing bytes to utf8")]
    Utf8,
    #[fail(display = "exmo client source - error parsing string to json")]
    Json,
    #[fail(display = "exmo client source - error during delay")]
    Delay,
    #[fail(display = "exmo client source - not enough amount on market")]
    NotEnoughAmount,
    #[fail(display = "exmo client source - no such currency conversion on market")]
    NoSuchConversion,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorContext {
    #[fail(display = "exmo client source - no data returned from graphql")]
    NoGraphQLData,
}

derive_error_impls!();
