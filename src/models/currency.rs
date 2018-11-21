use std::fmt::{self, Display};
use std::io::Write;
use std::str::FromStr;

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
    Usd,
    Rub,
}

impl FromSql<VarChar, Pg> for Currency {
    fn from_sql(data: Option<&[u8]>) -> deserialize::Result<Self> {
        match data {
            Some(b"eth") => Ok(Currency::Eth),
            Some(b"stq") => Ok(Currency::Stq),
            Some(b"btc") => Ok(Currency::Btc),
            Some(b"usd") => Ok(Currency::Usd),
            Some(b"rub") => Ok(Currency::Rub),
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
            Currency::Usd => out.write_all(b"usd")?,
            Currency::Rub => out.write_all(b"rub")?,
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
            Currency::Usd => f.write_str("usd"),
            Currency::Rub => f.write_str("rub"),
        }
    }
}

impl FromStr for Currency {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cur = match s {
            "eth" => Currency::Eth,
            "stq" => Currency::Stq,
            "btc" => Currency::Btc,
            "usd" => Currency::Usd,
            "rub" => Currency::Rub,
            _ => return Err(()),
        };
        Ok(cur)
    }
}

const MAX_RATE: f64 = 6500000.0;
pub const FIAT_DECIMALS: u128 = 1u128;
pub const BTC_DECIMALS: u128 = 100_000_000u128;
pub const ETH_DECIMALS: u128 = 1_000_000_000_000_000_000u128;
pub const STQ_DECIMALS: u128 = 1_000_000_000_000_000_000u128;

impl Currency {
    pub fn to_f64(self, value: Amount) -> f64 {
        let decimals = match self {
            Currency::Btc => BTC_DECIMALS,
            Currency::Eth => ETH_DECIMALS,
            Currency::Stq => STQ_DECIMALS,
            Currency::Usd => FIAT_DECIMALS,
            Currency::Rub => FIAT_DECIMALS,
        };
        // Max of all rates
        let max_rate = MAX_RATE as u128;
        // first multiply by max_rate and then divide by it
        // that is made so that we can use integer division of u128 (f64 is not enough)
        // and be sure that our error is less that 0,1 cent
        let mut crypto_value_times_rate: u128 = value.raw() * max_rate / decimals;
        // if value is less then 0,1 cent then we set it to 0,1 cent
        if crypto_value_times_rate == 0 {
            crypto_value_times_rate = 1;
        }
        // after dividing by decimals we have value small enough to be used as f64
        (crypto_value_times_rate as f64) / (max_rate as f64)
    }

    pub fn from_f64(self, value: f64) -> Amount {
        let decimals = match self {
            Currency::Btc => BTC_DECIMALS,
            Currency::Eth => ETH_DECIMALS,
            Currency::Stq => STQ_DECIMALS,
            Currency::Usd => FIAT_DECIMALS,
            Currency::Rub => FIAT_DECIMALS,
        };
        let val = value * MAX_RATE;
        let crypto_val = (val as u128) * decimals / (MAX_RATE as u128);
        Amount::new(crypto_val)
    }
}
