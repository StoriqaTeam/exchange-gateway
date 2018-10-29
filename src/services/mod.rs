mod auth;
mod error;
mod exchange;
#[cfg(test)]
mod mocks;
mod users;

pub use self::auth::*;
pub use self::error::*;
pub use self::exchange::*;
#[cfg(test)]
pub use self::mocks::*;
pub use self::users::*;

use prelude::*;

type ServiceFuture<T> = Box<Future<Item = T, Error = Error> + Send>;
