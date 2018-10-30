use std::fmt::{self, Display};
use std::io::Write;

use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::VarChar;

use models::*;

#[derive(Debug, Serialize, Deserialize, FromSqlRow, AsExpression, Clone, Copy, Eq, PartialEq, Hash)]
#[sql_type = "VarChar"]
#[serde(rename_all = "lowercase")]
pub enum Currency {
    Eth,
    Stq,
    Btc,
}

impl FromSql<VarChar, Pg> for Currency {
    fn from_sql(data: Option<&[u8]>) -> deserialize::Result<Self> {
        match data {
            Some(b"eth") => Ok(Currency::Eth),
            Some(b"stq") => Ok(Currency::Stq),
            Some(b"btc") => Ok(Currency::Btc),
            Some(v) => Err(format!(
                "Unrecognized enum variant: {:?}",
                String::from_utf8(v.to_vec()).unwrap_or_else(|_| "Non - UTF8 value".to_string())
            ).to_string()
            .into()),
            None => Err("Unexpected null for non-null column".into()),
        }
    }
}

impl ToSql<VarChar, Pg> for Currency {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match self {
            Currency::Eth => out.write_all(b"eth")?,
            Currency::Stq => out.write_all(b"stq")?,
            Currency::Btc => out.write_all(b"btc")?,
        };
        Ok(IsNull::No)
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Currency::Eth => f.write_str("eth"),
            Currency::Stq => f.write_str("stq"),
            Currency::Btc => f.write_str("btc"),
        }
    }
}

const MAX_RATE: f64 = 6500.0;
pub const BTC_DECIMALS: u128 = 100_000_000u128;
pub const ETH_DECIMALS: u128 = 1_000_000_000_000_000_000u128;
pub const STQ_DECIMALS: u128 = 1_000_000_000_000_000_000u128;

impl Currency {
    pub fn to_f64(self, value: Amount) -> f64 {
        let decimals = match self {
            Currency::Btc => BTC_DECIMALS,
            Currency::Eth => ETH_DECIMALS,
            Currency::Stq => STQ_DECIMALS,
        };
        // Max of all rates
        let max_rate = MAX_RATE as u128;
        // first multiply by max_rate and then divide by it
        // that is made so that we can use integer division of u128 (f64 is not enough)
        // and be sure that our error is less that 1 dollar
        let crypto_value_times_rate: u128 = value.raw() * max_rate / decimals;
        // after dividing by decimals we have value small enough to be used as f64
        (crypto_value_times_rate as f64) / (max_rate as f64)
    }
}
