use std::sync::{Arc, Mutex};

use super::error::*;
use super::exchange::*;
use super::executor::DbExecutor;
use super::types::RepoResult;
use models::*;
use prelude::*;

#[derive(Clone, Default)]
pub struct ExchangesRepoMock {
    data: Arc<Mutex<Vec<Exchange>>>,
}

impl ExchangesRepo for ExchangesRepoMock {
    fn create(&self, payload: NewExchange) -> Result<Exchange, Error> {
        let mut data = self.data.lock().unwrap();
        let res: Exchange = payload.into();
        data.push(res.clone());
        Ok(res)
    }
    fn get(&self, req: GetExchange) -> RepoResult<Option<Exchange>> {
        let data = self.data.lock().unwrap();
        Ok(data
            .iter()
            .filter(|x| {
                x.from_currency == req.from_currency && x.to_currency == req.to_currency && x.amount == req.amount && x.rate >= req.rate
            }).nth(0)
            .cloned())
    }
}

#[derive(Clone, Default)]
pub struct DbExecutorMock;

impl DbExecutor for DbExecutorMock {
    fn execute<F, T, E>(&self, f: F) -> Box<Future<Item = T, Error = E> + Send + 'static>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, E> + Send + 'static,
        E: From<Error> + Send + 'static,
    {
        Box::new(f().into_future())
    }
    fn execute_transaction<F, T, E>(&self, f: F) -> Box<Future<Item = T, Error = E> + Send + 'static>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, E> + Send + 'static,
        E: From<Error> + Send + 'static,
    {
        Box::new(f().into_future())
    }
    fn execute_test_transaction<F, T, E>(&self, f: F) -> Box<Future<Item = T, Error = E> + Send + 'static>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, E> + Send + 'static,
        E: From<Error> + Fail,
    {
        Box::new(f().into_future())
    }
}
