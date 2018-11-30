use std::fmt::{self, Display};
use std::num::ParseIntError;
use std::str::FromStr;

use diesel::sql_types::Int8 as SqlInt8;

#[derive(Debug, Serialize, Deserialize, FromSqlRow, AsExpression, Clone, Copy, Default, PartialEq)]
#[sql_type = "SqlInt8"]
pub struct Nonce(i64);
derive_newtype_sql!(nonce, SqlInt8, Nonce, Nonce);

impl Nonce {
    pub fn new(id: i64) -> Self {
        Nonce(id)
    }
    pub fn inner(&self) -> i64 {
        self.0
    }
    pub fn generate() -> Self {
        let now = ::chrono::Utc::now().naive_utc();
        let seconds = now.timestamp();
        let milis = now.timestamp_subsec_millis();
        Nonce((seconds * 1_000) + milis as i64)
    }
}

impl FromStr for Nonce {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = s.parse()?;
        Ok(Nonce::new(id))
    }
}

impl Display for Nonce {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("{}", self.0,))
    }
}
