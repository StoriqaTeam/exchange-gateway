use std::collections::HashMap;
use std::fmt;
use std::num::ParseFloatError;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use validator::{ValidationError, ValidationErrors};

use super::error::*;
use models::*;
use prelude::*;

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoCreateOrderResponse {
    pub result: bool,
    pub error: String,
    pub order_id: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoCancelOrderResponse {
    pub result: bool,
    pub error: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoOrderResponse {
    #[serde(rename = "type")]
    pub type_: String,
    pub in_currency: String,
    #[serde(deserialize_with = "string_to_f64")]
    pub in_amount: f64,
    pub out_currency: String,
    #[serde(deserialize_with = "string_to_f64")]
    pub out_amount: f64,
    pub trades: Vec<ExmoTrade>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoTrade {
    pub trade_id: u64,
    pub date: u64,
    #[serde(rename = "type")]
    pub type_: String,
    pub pair: String,
    pub order_id: u64,
    #[serde(deserialize_with = "string_to_f64")]
    pub quantity: f64,
    #[serde(deserialize_with = "string_to_f64")]
    pub price: f64,
    #[serde(deserialize_with = "string_to_f64")]
    pub amount: f64,
}

#[derive(Debug, Clone)]
pub struct ExmoGetExchangeResponse {
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl Default for ExmoGetExchangeResponse {
    fn default() -> Self {
        Self {
            from: Currency::Eth,
            to: Currency::Btc,
            amount: Amount::default(),
            rate: 0.33,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoRateResponse {
    #[serde(flatten)]
    pub pair: HashMap<String, ExmoBook>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoBook {
    #[serde(deserialize_with = "string_to_f64")]
    pub ask_quantity: f64, //ask_quantity - the sum of all quantity values in sell orders
    #[serde(deserialize_with = "string_to_f64")]
    pub ask_amount: f64, //ask_amount - the sum of all total sum values in sell orders
    #[serde(deserialize_with = "string_to_f64")]
    pub ask_top: f64, //ask_top - minimum sell price
    #[serde(deserialize_with = "string_to_f64")]
    pub bid_quantity: f64, //bid_quantity - the sum of all quantity values in buy orders
    #[serde(deserialize_with = "string_to_f64")]
    pub bid_amount: f64, //bid_amount - the sum of all total sum values in buy orders
    #[serde(deserialize_with = "string_to_f64")]
    pub bid_top: f64, //bid_top - maximum buy price
    pub ask: Vec<ExmoOrder>, //ask - the list of sell orders where every field is: price, quantity and amount
    pub bid: Vec<ExmoOrder>, //bid - the list of buy orders where every field is: price, quantity and amount
}

impl ExmoBook {
    /// to get right rate we first need to choose
    /// where to look for rates:
    /// if we want to buy, then we are watching sell orders.
    /// if we want to sell, then we are watching buy orders.
    /// Orders are ordered by price, therefore we need to get top orders
    /// with enough quantity, after that we calculate weighted average
    pub fn get_rate(&self, needed_quantity: f64, type_: OrderType) -> Result<f64, Error> {
        let orders = match type_ {
            OrderType::Buy | OrderType::SellTotal => self.ask.iter(),
            OrderType::Sell | OrderType::BuyTotal => self.bid.iter(),
        };
        let mut needed_orders = vec![];
        let mut total_quantity = 0f64;
        for order in orders {
            if total_quantity >= needed_quantity {
                break;
            }
            total_quantity += order.quantity;
            needed_orders.push(order);
        }
        if needed_quantity > total_quantity {
            let mut errors = ValidationErrors::new();
            let mut error = ValidationError::new("not_enough_on_market");
            error.message = Some("At the moment, the exchange of this amount is not possible, please try again later.".into());
            errors.add("value", error);
            return Err(ectx!(err ErrorContext::NotEnoughAmount, ErrorKind::InvalidInput(errors) => needed_quantity, total_quantity));
        }
        let total_amount = needed_orders.iter().fold(0f64, |mut amount, x| {
            amount += x.amount;
            amount
        });
        let weighted_average = total_amount / total_quantity;
        Ok(weighted_average)
    }
}

impl Default for ExmoBook {
    fn default() -> Self {
        Self {
            ask_quantity: 1f64,
            ask_amount: 1f64,
            ask_top: 1f64,
            bid_quantity: 1f64,
            bid_amount: 1f64,
            bid_top: 1f64,
            ask: vec![ExmoOrder::new(1f64, 1f64, 1f64)],
            bid: vec![ExmoOrder::new(1f64, 1f64, 1f64)],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExmoOrder {
    pub price: f64,
    pub quantity: f64,
    pub amount: f64,
}

impl ExmoOrder {
    pub fn new(price: f64, quantity: f64, amount: f64) -> Self {
        Self { price, quantity, amount }
    }
}

impl<'de> Deserialize<'de> for ExmoOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Price,
            Quantity,
            Amount,
        };

        struct ExmoOrderVisitor;

        impl<'de> Visitor<'de> for ExmoOrderVisitor {
            type Value = ExmoOrder;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ExmoOrder")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<ExmoOrder, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut value: String = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let price = value
                    .parse()
                    .map_err(|e: ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))?;
                value = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let quantity = value
                    .parse()
                    .map_err(|e: ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))?;
                value = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let amount = value
                    .parse()
                    .map_err(|e: ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))?;
                Ok(ExmoOrder::new(price, quantity, amount))
            }

            fn visit_map<V>(self, mut map: V) -> Result<ExmoOrder, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut price = None;
                let mut quantity = None;
                let mut amount = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Price => {
                            if price.is_some() {
                                return Err(de::Error::duplicate_field("price"));
                            }
                            price = Some(map.next_value()?);
                        }
                        Field::Quantity => {
                            if quantity.is_some() {
                                return Err(de::Error::duplicate_field("quantity"));
                            }
                            quantity = Some(map.next_value()?);
                        }
                        Field::Amount => {
                            if amount.is_some() {
                                return Err(de::Error::duplicate_field("amount"));
                            }
                            amount = Some(map.next_value()?);
                        }
                    }
                }
                let price = price.ok_or_else(|| de::Error::missing_field("price"))?;
                let quantity = quantity.ok_or_else(|| de::Error::missing_field("quantity"))?;
                let amount = amount.ok_or_else(|| de::Error::missing_field("amount"))?;
                Ok(ExmoOrder::new(price, quantity, amount))
            }
        }

        const FIELDS: &'static [&'static str] = &["price", "quantity", "amount"];
        deserializer.deserialize_struct("ExmoOrder", FIELDS, ExmoOrderVisitor)
    }
}

fn string_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringTof64;

    impl<'de> Visitor<'de> for StringTof64 {
        type Value = f64;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<f64, E>
        where
            E: de::Error,
        {
            value
                .parse()
                .map_err(|e: ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))
        }

        fn visit_map<M>(self, visitor: M) -> Result<f64, M::Error>
        where
            M: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(visitor))
        }
    }

    deserializer.deserialize_any(StringTof64)
}

fn deserialize_hash_str_to_f64<'de, D>(deserializer: D) -> Result<HashMap<String, f64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "string_to_f64")] f64);

    let v = HashMap::<String, Wrapper>::deserialize(deserializer)?;
    Ok(v.into_iter().map(|(k, Wrapper(v))| (k, v)).collect())
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoUserInfo {
    #[serde(deserialize_with = "deserialize_hash_str_to_f64")]
    pub balances: HashMap<String, f64>,
    #[serde(deserialize_with = "deserialize_hash_str_to_f64")]
    pub reserved: HashMap<String, f64>,
}

impl From<ExmoUserInfo> for Metrics {
    fn from(ex: ExmoUserInfo) -> Self {
        let balances = ex
            .balances
            .into_iter()
            .filter_map(|(s, value)| s.to_lowercase().parse::<Currency>().ok().map(|cur| (cur, value)))
            .collect();
        Self { balances }
    }
}

impl ExmoUserInfo {
    pub fn get_balance(&self, currency: Currency) -> f64 {
        self.balances.get(&currency.to_string().to_uppercase()).cloned().unwrap_or_default()
    }
}

impl Default for ExmoUserInfo {
    fn default() -> Self {
        let mut balances = HashMap::new();
        balances.insert("BTC".to_string(), 100000f64);
        balances.insert("ETH".to_string(), 100000f64);
        balances.insert("STQ".to_string(), 100000000f64);
        let mut reserved = HashMap::new();
        reserved.insert("BTC".to_string(), 1f64);
        reserved.insert("ETH".to_string(), 1f64);
        reserved.insert("STQ".to_string(), 1000f64);
        Self { balances, reserved }
    }
}
