use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::error::*;
use super::exchange::*;
use super::executor::{DbExecutor, Isolation};
use super::sell_orders::*;
use super::types::RepoResult;
use super::users::*;
use models::*;
use prelude::*;

#[derive(Clone, Default)]
pub struct UsersRepoMock {
    data: Arc<Mutex<Vec<User>>>,
}

impl UsersRepo for UsersRepoMock {
    fn find_user_by_authentication_token(&self, token: AuthenticationToken) -> Result<Option<User>, Error> {
        let data = self.data.lock().unwrap();
        Ok(data.iter().filter(|x| x.authentication_token == token).nth(0).cloned())
    }

    fn create(&self, payload: NewUser) -> Result<User, Error> {
        let mut data = self.data.lock().unwrap();
        let res = User {
            id: payload.id,
            name: payload.name,
            authentication_token: payload.authentication_token,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
        };
        data.push(res.clone());
        Ok(res)
    }
    fn get(&self, user_id: UserId) -> RepoResult<Option<User>> {
        let data = self.data.lock().unwrap();
        Ok(data.iter().filter(|x| x.id == user_id).nth(0).cloned())
    }
    fn update(&self, user_id: UserId, payload: UpdateUser) -> RepoResult<User> {
        let mut data = self.data.lock().unwrap();
        let u = data
            .iter_mut()
            .filter_map(|x| {
                if x.id == user_id {
                    if let Some(ref name) = payload.name {
                        x.name = name.clone();
                    }
                    if let Some(ref authentication_token) = payload.authentication_token {
                        x.authentication_token = authentication_token.clone();
                    }
                    Some(x)
                } else {
                    None
                }
            }).nth(0)
            .cloned();
        Ok(u.unwrap())
    }
    fn delete(&self, user_id: UserId) -> RepoResult<User> {
        let data = self.data.lock().unwrap();
        Ok(data.iter().filter(|x| x.id == user_id).nth(0).cloned().unwrap())
    }
}

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
            .filter(|x| x.from_ == req.from && x.to_ == req.to && x.id == req.id)
            .nth(0)
            .cloned())
    }
}

#[derive(Clone, Default)]
pub struct SellOrdersRepoMock {
    data: Arc<Mutex<Vec<SellOrderDB>>>,
}

impl SellOrdersRepo for SellOrdersRepoMock {
    fn create(&self, payload: NewSellOrder) -> RepoResult<SellOrderDB> {
        let mut data = self.data.lock().unwrap();
        let res: SellOrderDB = payload.into();
        data.push(res.clone());
        Ok(res)
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
    fn execute_transaction_with_isolation<F, T, E>(&self, _isolation: Isolation, f: F) -> Box<Future<Item = T, Error = E> + Send + 'static>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, E> + Send + 'static,
        E: From<Error> + Fail,
    {
        Box::new(f().into_future())
    }
}
