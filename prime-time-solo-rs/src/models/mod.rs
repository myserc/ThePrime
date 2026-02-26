use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub id: u32,
    pub name: String,
    pub vault_books: f64,
    pub active_book: Option<ActiveBook>,
    pub counts: f64,
    pub prime_value: u32,
    pub balance_adjustment: i32,
    pub active_heuristic: Option<String>,
    pub last_receipt: Option<String>,
    pub is_online: bool,
    pub joined_at: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveBook {
    pub remaining_counts: f64,
    pub max_counts: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub id: f64,
    pub owner: String,
    pub value: f64,
    pub type_: String,
    pub time: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub id: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    pub counts: u32,
    pub color_class: &'static str,
    pub bg_class: &'static str,
}

pub const UNITS: [Unit; 5] = [
    Unit { id: "QUADRANT", name: "Quadrant", symbol: "◴", counts: 162000, color_class: "text-rose-400", bg_class: "bg-rose-500" },
    Unit { id: "DAY",      name: "Day",      symbol: "☼", counts: 43200,  color_class: "text-yellow-400", bg_class: "bg-yellow-500" },
    Unit { id: "DEGREE",   name: "Degree",   symbol: "°", counts: 1800,   color_class: "text-purple-400", bg_class: "bg-purple-500" },
    Unit { id: "MINUTE",   name: "Minute",   symbol: "'", counts: 30,     color_class: "text-indigo-400", bg_class: "bg-indigo-500" },
    Unit { id: "TWIN",     name: "Twin",     symbol: "♊", counts: 1,      color_class: "text-cyan-400", bg_class: "bg-cyan-500" },
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeuristicStandard {
    pub mint_scarcity: u32,
    pub mint_counts: usize,
    pub entropy_change: i32,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub total_book_counts: u32,
    pub time_scale: u32,
    pub prime_limit: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            total_book_counts: 648000,
            time_scale: 1,
            prime_limit: 10000000,
        }
    }
}
