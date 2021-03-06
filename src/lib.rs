#![allow(proc_macro_derive_resolution_fallback)]

extern crate chrono;
extern crate crypto;
extern crate failure;
extern crate futures;
#[macro_use]
extern crate diesel;
extern crate base64;
extern crate futures_cpupool;
extern crate hyper;
extern crate r2d2;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate serde_qs;
#[macro_use]
extern crate serde_derive;
extern crate time;
#[macro_use]
extern crate log;
extern crate config as config_crate;
#[macro_use]
extern crate http_router;
extern crate hyper_tls;
extern crate regex;
#[macro_use]
extern crate validator_derive;
extern crate num;
extern crate validator;
#[macro_use]
extern crate sentry;
extern crate env_logger;
extern crate futures_retry;
extern crate gelf;
extern crate simplelog;
extern crate tokio_core;
extern crate tokio_timer;
extern crate uuid;

#[macro_use]
mod macros;
mod api;
mod client;
mod config;
mod logger;
mod models;
mod prelude;
mod repos;
mod schema;
mod sentry_integration;
mod services;
mod utils;

use config::Config;

use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use futures_cpupool::CpuPool;

use self::models::NewUser;
use self::prelude::*;
use self::repos::{DbExecutor, DbExecutorImpl, Error as ReposError, UsersRepo, UsersRepoImpl};

pub fn hello() {
    println!("Hello world");
}

pub fn print_config() {
    println!("Parsed config: {:?}", get_config());
}

pub fn start_server() {
    let config = get_config();
    // Prepare sentry integration
    let _sentry = sentry_integration::init(config.sentry.as_ref());
    // Prepare logger
    logger::init(&config);

    api::start_server(config);
}

fn get_config() -> Config {
    config::Config::new().unwrap_or_else(|e| panic!("Error parsing config: {}", e))
}

pub fn create_user(name: &str) {
    let config = get_config();
    let db_pool = create_db_pool(&config);
    let cpu_pool = CpuPool::new(1);
    let users_repo = UsersRepoImpl;
    let db_executor = DbExecutorImpl::new(db_pool, cpu_pool);
    let mut new_user: NewUser = Default::default();
    new_user.name = name.to_string();
    let fut = db_executor.execute(move || -> Result<(), ReposError> {
        let user = users_repo.create(new_user).expect("Failed to create user");
        println!("{}", user.authentication_token.raw());
        Ok(())
    });
    hyper::rt::run(fut.map(|_| ()).map_err(|_| ()));
}

fn create_db_pool(config: &Config) -> PgPool {
    let database_url = config.database.url.clone();
    let manager = ConnectionManager::<PgConnection>::new(database_url.clone());
    r2d2::Pool::builder()
        .build(manager)
        .unwrap_or_else(|_| panic!("Failed to connect to db with url: {}", database_url))
}
