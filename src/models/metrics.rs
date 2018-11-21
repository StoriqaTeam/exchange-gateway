use std::collections::HashMap;

use models::*;

#[derive(Debug, Clone)]
pub struct Metrics {
    pub balances: HashMap<Currency, f64>,
}
