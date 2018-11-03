use std::fmt::{self, Display};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Hash)]
pub enum OrderType {
    Sell,
    Buy,
    SellTotal,
    BuyTotal,
}

impl Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderType::Sell => f.write_str("market_sell"),
            OrderType::Buy => f.write_str("market_buy"),
            OrderType::SellTotal => f.write_str("market_sell_total"),
            OrderType::BuyTotal => f.write_str("market_buy_total"),
        }
    }
}
