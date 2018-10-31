use std::fmt::{self, Display};
use std::num::ParseIntError;
use std::str::FromStr;

use diesel::sql_types::Int4 as SqlInt4;

#[derive(Debug, Serialize, Deserialize, FromSqlRow, AsExpression, Clone, Copy, Default, PartialEq)]
#[sql_type = "SqlInt4"]
pub struct Nonce(i32);
derive_newtype_sql!(nonce, SqlInt4, Nonce, Nonce);

impl Nonce {
    pub fn new(id: i32) -> Self {
        Nonce(id)
    }
    pub fn inner(&self) -> i32 {
        self.0
    }
    pub fn generate() -> Self {
        Nonce(i32::default())
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
