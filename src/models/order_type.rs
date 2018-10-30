use std::fmt::{self, Display};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Hash)]
pub enum OrderType {
    Sell,
    Buy,
}

impl Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderType::Sell => f.write_str("sell"),
            OrderType::Buy => f.write_str("buy"),
        }
    }
}
