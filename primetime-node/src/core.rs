use lazy_static::lazy_static;
use std::collections::HashMap;

pub const LIMIT: usize = 15_000_000;
pub const TOTAL_BOOK_COUNTS: i64 = 648_000;
pub const STANDARD_MINT_SCARCITY: u64 = 9_731_081;

lazy_static! {
    pub static ref PRIMES: Vec<u64> = generate_primes(LIMIT);
}

pub fn generate_primes(limit: usize) -> Vec<u64> {
    let mut sieve = vec![true; limit];
    let mut primes = Vec::with_capacity(limit / 10);
    for p in 2..limit {
        if sieve[p] {
            primes.push(p as u64);
            let mut i = p * p;
            while i < limit {
                sieve[i] = false;
                i += p;
            }
        }
    }
    primes
}

pub fn get_ordinal_for_prime(value: u64) -> usize {
    let idx = PRIMES.partition_point(|&x| x <= value);
    if idx == 0 { 0 } else { idx - 1 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Unit { Quadrant, Day, Degree, Minute, Twin }

impl Unit {
    pub fn counts(&self) -> usize {
        match self {
            Unit::Quadrant => 162_000,
            Unit::Day => 43_200,
            Unit::Degree => 1_800,
            Unit::Minute => 30,
            Unit::Twin => 1,
        }
    }

    pub fn all() -> Vec<Unit> {
        vec![Unit::Day, Unit::Degree, Unit::Twin]
    }
}

pub struct HeuristicStandard {
    pub mint_scarcity: u64,
    pub mint_counts: usize,
    pub precedent: u64,
}

lazy_static! {
    pub static ref STANDARDS: HashMap<Unit, HeuristicStandard> = {
        let mut map = HashMap::new();
        // Force evaluation of PRIMES
        let _ = &*PRIMES;
        for unit in Unit::all() {
            let precedent = PRIMES[unit.counts() - 1];
            let result1 = STANDARD_MINT_SCARCITY / precedent;
            let mint_scarcity = result1 * precedent;
            let mint_counts = get_ordinal_for_prime(mint_scarcity) + 1;
            map.insert(unit, HeuristicStandard {
                mint_scarcity,
                mint_counts,
                precedent,
            });
        }
        map
    };
}
