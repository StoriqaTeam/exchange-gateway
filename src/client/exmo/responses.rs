use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use super::error::*;
use models::*;
use prelude::*;
#[derive(Debug, Clone)]
pub struct ExmoCreateSellOrder {
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl From<Exchange> for ExmoCreateSellOrder {
    fn from(sell: Exchange) -> Self {
        Self {
            from: sell.from_,
            to: sell.to_,
            amount: sell.amount,
            rate: sell.rate,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExmoSellOrderResponse {
    pub id: BlockchainTransactionId,
    pub from: Currency,
    pub to: Currency,
    pub amount: Amount,
    pub rate: f64,
}

impl From<ExmoSellOrderResponse> for SellOrder {
    fn from(sell: ExmoSellOrderResponse) -> Self {
        Self {
            id: sell.id,
            from: sell.from,
            to: sell.to,
            amount: sell.amount,
            rate: sell.rate,
        }
    }
}

impl Default for ExmoSellOrderResponse {
    fn default() -> Self {
        Self {
            id: BlockchainTransactionId::default(),
            from: Currency::Eth,
            to: Currency::Btc,
            amount: Amount::default(),
            rate: f64::default(),
        }
    }
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

#[derive(Debug, Deserialize, Clone, Default)]
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
    pub fn get_rate(&self, needed_amount: f64, type_: OrderType) -> Result<f64, Error> {
        let orders = match type_ {
            OrderType::Buy => self.ask.iter(),
            OrderType::Sell => self.bid.iter(),
        };
        let (amount_average, price_average, quantity_average) = orders.fold((0f64, 0f64, 0f64), |acc, x| {
            let (mut amount, mut price, mut quantity) = acc;
            amount += x.amount;
            price += x.price;
            quantity += x.quantity;
            (amount, price, quantity)
        });
        if needed_amount > quantity_average {
            return Err(ectx!(err ErrorSource::NotEnoughAmount, ErrorKind::Internal => needed_amount, quantity_average));
        }
        let weighted_average = amount_average / price_average;
        Ok(weighted_average)
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
                    .map_err(|e: std::num::ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))?;
                value = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let quantity = value
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))?;
                value = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let amount = value
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))?;
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
                .map_err(|e: std::num::ParseFloatError| de::Error::invalid_type(de::Unexpected::Other(&e.to_string()), &self))
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
