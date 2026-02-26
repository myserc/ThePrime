use wasm_bindgen::prelude::*;
use std::collections::HashMap;
use crate::models::{Config, HeuristicStandard, UNITS};

pub struct Engine {
    pub primes: Vec<u32>,
    pub precedents: HashMap<String, u32>,
    pub heuristic_standards: HashMap<String, HeuristicStandard>,
    pub standard_mint_scarcity: u32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            primes: Vec::new(),
            precedents: HashMap::new(),
            heuristic_standards: HashMap::new(),
            standard_mint_scarcity: 0,
        }
    }

    pub fn init(&mut self, config: &Config) {
        let limit = config.prime_limit as usize;
        let mut sieve = vec![true; limit];
        sieve[0] = false;
        sieve[1] = false;

        // Simple sieve
        for i in 2..((limit as f64).sqrt() as usize + 1) {
            if sieve[i] {
                let mut j = i * i;
                while j < limit {
                    sieve[j] = false;
                    j += i;
                }
            }
        }

        self.primes = sieve.iter().enumerate()
            .filter_map(|(i, &is_prime)| if is_prime { Some(i as u32) } else { None })
            .collect();

        // Calculate Precedents
        for unit in UNITS.iter() {
            let prime = self.get_prime_at(unit.counts as usize - 1);
            self.precedents.insert(unit.id.to_string(), prime);
        }

        self.standard_mint_scarcity = self.get_prime_at(config.total_book_counts as usize - 1);

        for unit in UNITS.iter() {
            let precedent = *self.precedents.get(unit.id).unwrap_or(&1);
            if precedent == 0 { continue; }

            let result1 = (self.standard_mint_scarcity as f64 / precedent as f64).floor() as u32;
            let result2 = result1 * precedent;

            let result3 = self.get_ordinal_for_prime(result2) + 1;
            let result4 = config.total_book_counts as i32 - result3 as i32;

            self.heuristic_standards.insert(unit.id.to_string(), HeuristicStandard {
                mint_scarcity: result2,
                mint_counts: result3,
                entropy_change: result4,
            });
        }
    }

    pub fn get_prime_at(&self, index: usize) -> u32 {
        if index >= self.primes.len() {
            *self.primes.last().unwrap_or(&0)
        } else {
            self.primes[index]
        }
    }

    pub fn get_ordinal_for_prime(&self, value: u32) -> usize {
        match self.primes.binary_search(&value) {
            Ok(index) => index,
            Err(index) => if index > 0 { index - 1 } else { 0 },
        }
    }
}
