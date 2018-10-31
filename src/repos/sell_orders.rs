use diesel;

use super::error::*;
use super::executor::with_tls_connection;
use super::*;
use models::*;
use prelude::*;
use schema::sell_orders::dsl::*;

pub trait SellOrdersRepo: Send + Sync + 'static {
    fn create(&self, payload: NewSellOrder) -> RepoResult<SellOrderDB>;
}

#[derive(Clone, Default)]
pub struct SellOrdersRepoImpl;

impl<'a> SellOrdersRepo for SellOrdersRepoImpl {
    fn create(&self, payload: NewSellOrder) -> RepoResult<SellOrderDB> {
        with_tls_connection(|conn| {
            diesel::insert_into(sell_orders)
                .values(payload.clone())
                .get_result::<SellOrderDB>(conn)
                .map_err(move |e| {
                    let error_kind = ErrorKind::from(&e);
                    ectx!(err e, error_kind => payload)
                })
        })
    }
}

#[cfg(test)]
pub mod tests {
    use diesel::r2d2::ConnectionManager;
    use diesel::PgConnection;
    use futures_cpupool::CpuPool;
    use r2d2;
    use tokio_core::reactor::Core;

    use super::*;
    use config::Config;
    use repos::DbExecutorImpl;

    fn create_executor() -> DbExecutorImpl {
        let config = Config::new().unwrap();
        let manager = ConnectionManager::<PgConnection>::new(config.database.url);
        let db_pool = r2d2::Pool::builder().build(manager).unwrap();
        let cpu_pool = CpuPool::new(1);
        DbExecutorImpl::new(db_pool.clone(), cpu_pool.clone())
    }

    #[test]
    fn sell_orders_create() {
        let mut core = Core::new().unwrap();
        let db_executor = create_executor();
        let sell_orders_repo = SellOrdersRepoImpl::default();
        let _ = core.run(db_executor.execute_test_transaction(move || {
            let new_exchange = NewSellOrder::default();
            let res = sell_orders_repo.create(new_exchange);
            assert!(res.is_ok());
            res
        }));
    }

}
