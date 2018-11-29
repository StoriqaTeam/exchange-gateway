use std::fmt;
use std::fmt::Display;

use failure::{Backtrace, Context, Fail};
use validator::ValidationErrors;

use client::exmo::ErrorKind as ExmoClientErrorKind;
use repos::{Error as ReposError, ErrorKind as ReposErrorKind};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "service error - unauthorized")]
    Unauthorized,
    #[fail(display = "service error - malformed input")]
    MalformedInput,
    #[fail(display = "service error - invalid input, errors: {}", _0)]
    InvalidInput(ValidationErrors),
    #[fail(display = "service error - internal error")]
    Internal,
    #[fail(display = "service error - not found")]
    NotFound,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorContext {
    #[fail(display = "service error context - internal error")]
    Internal,
    #[fail(display = "service error context - no exchange rate found")]
    NoExchangeRate,
    #[fail(display = "service error context - no such exchange rate on market")]
    NoSuchRate,
    #[fail(display = "service error context - invalid auth token")]
    InvalidToken,
    #[fail(display = "service error context - not enough amount on users balance in exmo")]
    NotEnoughCurrencyBalance,
}

derive_error_impls!();

impl From<ReposError> for Error {
    fn from(e: ReposError) -> Error {
        let kind: ErrorKind = e.kind().into();
        e.context(kind).into()
    }
}

impl From<ReposErrorKind> for ErrorKind {
    fn from(e: ReposErrorKind) -> ErrorKind {
        match e {
            ReposErrorKind::Internal => ErrorKind::Internal,
            ReposErrorKind::Unauthorized => ErrorKind::Unauthorized,
            ReposErrorKind::Constraints(validation_errors) => ErrorKind::InvalidInput(validation_errors),
        }
    }
}

impl From<ExmoClientErrorKind> for ErrorKind {
    fn from(err: ExmoClientErrorKind) -> Self {
        match err {
            ExmoClientErrorKind::Internal => ErrorKind::Internal,
            ExmoClientErrorKind::Unauthorized => ErrorKind::Unauthorized,
            ExmoClientErrorKind::MalformedInput => ErrorKind::MalformedInput,
            ExmoClientErrorKind::InvalidInput(validation_errors) => ErrorKind::InvalidInput(validation_errors),
        }
    }
}
