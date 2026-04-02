// ==========================================
// 1. IMPORTS & DEPENDENCIES
// ==========================================
use crossbeam::channel::{Receiver, Sender, unbounded};
use krabmaga::engine::agent::Agent;
use krabmaga::engine::schedule::Schedule;
use krabmaga::engine::state::State;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::Rng;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

// ==========================================
// 2. PRIME NUMBER SCARCITY ENGINE
// ==========================================
pub const LIMIT: usize = 10_000_000;
pub const TOTAL_BOOK_COUNTS: i64 = 648_000;
pub const STANDARD_MINT_SCARCITY: u64 = 9_731_081;

lazy_static! {
    pub static ref PRIMES: Vec<u64> = generate_primes(LIMIT);
    // ARCHITECTURE_SUGGESTIONS: O(1) Memory-efficient prime counting array up to LIMIT
    // This removes the need for O(log N) binary search millions of times per tick.
    pub static ref PRIME_PI: Vec<usize> = build_prime_pi(&PRIMES, LIMIT);
}

fn generate_primes(limit: usize) -> Vec<u64> {
    println!("Initializing Prime Sieve up to {}...", limit);
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
    println!("Engine Ready. Generated {} primes.", primes.len());
    primes
}

// Build a lookup table where PRIME_PI[x] gives the number of primes <= x
fn build_prime_pi(primes: &[u64], limit: usize) -> Vec<usize> {
    println!("Building Prime Pi Lookup Table...");
    let mut pi = vec![0; limit + 1];
    let mut count = 0;
    let mut prime_idx = 0;

    for i in 0..=limit {
        if prime_idx < primes.len() && i as u64 == primes[prime_idx] {
            count += 1;
            prime_idx += 1;
        }
        pi[i] = count;
    }
    pi
}

pub fn get_ordinal_for_prime(value: u64) -> usize {
    if value <= LIMIT as u64 {
        let count = PRIME_PI[value as usize];
        if count == 0 { 0 } else { count - 1 }
    } else {
        // Fallback to binary search if value exceeds pre-computed limits
        let idx = PRIMES.partition_point(|&x| x <= value);
        if idx == 0 { 0 } else { idx - 1 }
    }
}

// ==========================================
// 3. CONFIGURATION & HEURISTICS
// ==========================================
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Unit {
    Day,
    Degree,
    Twin,
}

impl Unit {
    pub fn counts(&self) -> usize {
        match self {
            Unit::Day => 1_800,
            Unit::Degree => 30,
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
        for unit in Unit::all() {
            let precedent = PRIMES[unit.counts() - 1];
            let result1 = STANDARD_MINT_SCARCITY / precedent;
            let mint_scarcity = result1 * precedent;
            let mint_counts = get_ordinal_for_prime(mint_scarcity) + 1;
            map.insert(
                unit,
                HeuristicStandard {
                    mint_scarcity,
                    mint_counts,
                    precedent,
                },
            );
        }
        map
    };
}

// Messages for Lock-Free Heuristic Transfers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferIntent {
    pub source_id: u32,
    pub target_id: u32,
    pub unit: Unit,
    pub amount: u64,
}

// ==========================================
// 4. THE ENGINE (MODEL/STATE)
// ==========================================
pub struct PrimeTimeModel {
    pub step: u64,
    pub agents: Vec<NodeAgent>,
    pub net_entropy: Arc<AtomicI64>,
    pub system_surplus: Arc<AtomicI64>,
    pub void_events: Arc<AtomicI64>,
    pub surplus_events: Arc<AtomicI64>,
    pub total_wealth: Arc<AtomicU64>,
    pub total_vault_books: Arc<AtomicU64>,
    pub books_standard: Arc<AtomicU64>,
    pub books_heuristic: Arc<AtomicU64>,

    // ARCHITECTURE_SUGGESTIONS: Lock-free Transfer intents queue
    pub transfer_tx: Sender<TransferIntent>,
    pub transfer_rx: Receiver<TransferIntent>,
}

impl PrimeTimeModel {
    pub fn new(num_agents: u32) -> (Self, Schedule) {
        let mut schedule = Schedule::new();
        let (tx, rx) = unbounded();

        let mut model = PrimeTimeModel {
            step: 0,
            agents: Vec::with_capacity(num_agents as usize),
            net_entropy: Arc::new(AtomicI64::new(0)),
            system_surplus: Arc::new(AtomicI64::new(0)),
            void_events: Arc::new(AtomicI64::new(0)),
            surplus_events: Arc::new(AtomicI64::new(0)),
            total_wealth: Arc::new(AtomicU64::new(0)),
            total_vault_books: Arc::new(AtomicU64::new(0)),
            books_standard: Arc::new(AtomicU64::new(0)),
            books_heuristic: Arc::new(AtomicU64::new(0)),

            transfer_tx: tx,
            transfer_rx: rx,
        };

        let mut rng = rand::thread_rng();
        let mut total_initial_books = 0;

        for id in 0..num_agents {
            let counts = rng.gen_range(1..=5000);
            let agent = NodeAgent::new(id, counts);
            total_initial_books += agent.data.lock().vault_books;
            model.agents.push(agent.clone());
            schedule.schedule_repeating(Box::new(agent), 0.0, 0);
        }

        model
            .total_vault_books
            .store(total_initial_books, Ordering::Relaxed);
        model
            .books_standard
            .store(total_initial_books, Ordering::Relaxed);

        (model, schedule)
    }

    pub fn resolve_transfers(&mut self) {
        let mut intents = Vec::new();
        while let Ok(intent) = self.transfer_rx.try_recv() {
            intents.push(intent);
        }

        let mut total_entropy_leap = 0;
        let num_agents = self.agents.len();

        for intent in intents {
            if intent.source_id == intent.target_id {
                continue;
            }
            if (intent.source_id as usize) >= num_agents
                || (intent.target_id as usize) >= num_agents
            {
                continue;
            }

            // O(1) direct indexing instead of O(N) iter().find()
            let source_agent = &self.agents[intent.source_id as usize];
            let target_agent = &self.agents[intent.target_id as usize];

            let mut my_data = source_agent.data.lock();
            let mut target_data = target_agent.data.lock();

            let std = &STANDARDS[&intent.unit];
            let total_val = intent.amount * std.precedent;

            if my_data.prime_value < total_val {
                continue;
            }

            let old_source_counts = my_data.counts;
            let old_target_counts = target_data.counts;

            let new_source_val = my_data.prime_value.saturating_sub(total_val);
            let new_source_counts_idx = get_ordinal_for_prime(new_source_val);
            let new_source_base = PRIMES[new_source_counts_idx];

            my_data.prime_value = new_source_val;
            my_data.counts = new_source_counts_idx + 1;
            my_data.balance_adjustment = new_source_val.saturating_sub(new_source_base);

            target_data.prime_value += total_val;
            target_data.counts = get_ordinal_for_prime(target_data.prime_value) + 1;
            let new_target_base = PRIMES[target_data.counts - 1];
            target_data.balance_adjustment =
                target_data.prime_value.saturating_sub(new_target_base);

            target_data.active_heuristic = Some(intent.unit);

            let source_leap = my_data.counts as i64 - old_source_counts as i64;
            let target_leap = target_data.counts as i64 - old_target_counts as i64;
            let net_leap = source_leap + target_leap;

            total_entropy_leap += net_leap;

            my_data.entropy_delta += source_leap;
            target_data.entropy_delta += target_leap;
        }

        self.net_entropy
            .fetch_add(total_entropy_leap, Ordering::Relaxed);
    }
}

impl State for PrimeTimeModel {
    fn update(&mut self, _step: u64) {
        // ARCHITECTURE_SUGGESTIONS: State Contention
        // Aggregate local accumulators from agent level to global atomics
        // to completely eliminate false-sharing in the concurrent `step` phase.
        let mut added_entropy = 0;
        let mut added_vault_books = 0;
        let mut added_standard = 0;
        let mut added_heuristic = 0;

        for agent in &self.agents {
            let mut data = agent.data.lock();
            added_entropy += data.local_entropy_acc;
            added_vault_books += data.local_vault_acc;
            added_standard += data.local_std_acc;
            added_heuristic += data.local_heu_acc;

            // Reset agent local accumulators for the next tick
            data.local_entropy_acc = 0;
            data.local_vault_acc = 0;
            data.local_std_acc = 0;
            data.local_heu_acc = 0;
        }

        if added_vault_books > 0 {
            self.total_vault_books
                .fetch_add(added_vault_books, Ordering::Relaxed);
        }
        if added_standard > 0 {
            self.books_standard
                .fetch_add(added_standard, Ordering::Relaxed);
        }
        if added_heuristic > 0 {
            self.books_heuristic
                .fetch_add(added_heuristic, Ordering::Relaxed);
        }

        let mut current_entropy =
            self.net_entropy.fetch_add(added_entropy, Ordering::Relaxed) + added_entropy;

        // Resolve lock-free transfers
        self.resolve_transfers();
        current_entropy = self.net_entropy.load(Ordering::Relaxed);

        let mut surplus = self.system_surplus.load(Ordering::Relaxed);

        while current_entropy >= TOTAL_BOOK_COUNTS {
            current_entropy -= TOTAL_BOOK_COUNTS;
            surplus += 1;
            self.surplus_events.fetch_add(1, Ordering::Relaxed);
        }

        while current_entropy <= -TOTAL_BOOK_COUNTS {
            current_entropy += TOTAL_BOOK_COUNTS;
            self.void_events.fetch_add(1, Ordering::Relaxed);
            if surplus > 0 {
                surplus -= 1;
            }
        }

        self.net_entropy.store(current_entropy, Ordering::Relaxed);
        self.system_surplus.store(surplus, Ordering::Relaxed);

        self.step += 1;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn as_state(&self) -> &dyn State {
        self
    }
    fn as_state_mut(&mut self) -> &mut dyn State {
        self
    }
    fn init(&mut self, _schedule: &mut Schedule) {}
    fn reset(&mut self) {
        self.step = 0;
    }
}

// ==========================================
// 5. AGENT (THE NODE)
// ==========================================
#[derive(Clone)]
pub struct NodeAgent {
    pub id: u32,
    pub data: Arc<Mutex<NodeAgentData>>,
}

// Added deriving traits to enable checkpointing
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeAgentData {
    pub vault_books: u64,
    pub counts: usize,
    pub prime_value: u64,
    pub balance_adjustment: u64,
    pub active_heuristic: Option<Unit>,
    pub entropy_delta: i64,

    // Agent-local accumulators to eliminate global Mutex/Atomic false sharing
    pub local_entropy_acc: i64,
    pub local_vault_acc: u64,
    pub local_std_acc: u64,
    pub local_heu_acc: u64,
}

impl NodeAgent {
    pub fn new(id: u32, initial_counts: usize) -> Self {
        let mut data = NodeAgentData {
            vault_books: 2,
            counts: initial_counts,
            prime_value: 2,
            balance_adjustment: 0,
            active_heuristic: None,
            entropy_delta: 0,

            local_entropy_acc: 0,
            local_vault_acc: 0,
            local_std_acc: 0,
            local_heu_acc: 0,
        };
        data.update_prime_value();

        NodeAgent {
            id,
            data: Arc::new(Mutex::new(data)),
        }
    }
}

impl NodeAgentData {
    fn update_prime_value(&mut self) {
        let mut ordinal_idx = if self.counts > 0 { self.counts - 1 } else { 0 };
        ordinal_idx = ordinal_idx.min(PRIMES.len() - 1);
        self.prime_value = PRIMES[ordinal_idx] + self.balance_adjustment;
    }
}

impl Agent for NodeAgent {
    fn step(&mut self, state: &mut dyn State) {
        let state = state.as_any_mut().downcast_mut::<PrimeTimeModel>().unwrap();
        let mut rng = rand::thread_rng();

        let mut my_data = self.data.lock();

        // 1. ROLLING MOMENTUM
        my_data.counts += rng.gen_range(10..=200);
        my_data.update_prime_value();

        // 2. MINT CHECK
        let (threshold, counts_consumed) = if let Some(h) = my_data.active_heuristic {
            let std = &STANDARDS[&h];
            (std.mint_scarcity, std.mint_counts)
        } else {
            (STANDARD_MINT_SCARCITY, TOTAL_BOOK_COUNTS as usize)
        };

        if my_data.prime_value >= threshold {
            let efficiency_gain = TOTAL_BOOK_COUNTS - counts_consumed as i64;
            if efficiency_gain > 0 {
                // Store in agent-local accumulators
                my_data.local_entropy_acc += efficiency_gain;
                my_data.entropy_delta += efficiency_gain;
            }
            my_data.vault_books += 1;

            if my_data.active_heuristic.is_some() {
                my_data.local_heu_acc += 1;
            } else {
                my_data.local_std_acc += 1;
            }
            my_data.local_vault_acc += 1;

            my_data.counts = 1.max(my_data.counts.saturating_sub(counts_consumed));
            my_data.balance_adjustment = 0;
            my_data.active_heuristic = None;
            my_data.update_prime_value();
        }

        // 3. HEURISTIC TRANSFER (Intent queued instead of try_lock)
        if rng.gen_bool(0.05) {
            let target_agent = state.agents.choose(&mut rng).unwrap();
            if target_agent.id == self.id {
                return;
            }

            let affordable: Vec<Unit> = Unit::all()
                .into_iter()
                .filter(|u| my_data.prime_value >= STANDARDS[u].precedent)
                .collect();

            if affordable.is_empty() {
                return;
            }
            let unit = affordable.choose(&mut rng).unwrap();
            let std = &STANDARDS[unit];

            let max_amount = my_data.prime_value / std.precedent;
            let amount = rng.gen_range(1..=max_amount);

            // Queue the transfer intent
            let _ = state.transfer_tx.send(TransferIntent {
                source_id: self.id,
                target_id: target_agent.id,
                unit: *unit,
                amount,
            });
        }
    }
}

// Checkpointing Serializable Structures
#[derive(Clone, Serialize, Deserialize)]
pub struct EngineCheckpoint {
    pub step: u64,
    pub net_entropy: i64,
    pub system_surplus: i64,
    pub void_events: i64,
    pub surplus_events: i64,
    pub total_wealth: u64,
    pub total_vault_books: u64,
    pub books_standard: u64,
    pub books_heuristic: u64,
    pub agents: Vec<(u32, NodeAgentData)>,
}

impl PrimeTimeModel {
    pub fn save_checkpoint(&self) -> EngineCheckpoint {
        EngineCheckpoint {
            step: self.step,
            net_entropy: self.net_entropy.load(Ordering::Relaxed),
            system_surplus: self.system_surplus.load(Ordering::Relaxed),
            void_events: self.void_events.load(Ordering::Relaxed),
            surplus_events: self.surplus_events.load(Ordering::Relaxed),
            total_wealth: self.total_wealth.load(Ordering::Relaxed),
            total_vault_books: self.total_vault_books.load(Ordering::Relaxed),
            books_standard: self.books_standard.load(Ordering::Relaxed),
            books_heuristic: self.books_heuristic.load(Ordering::Relaxed),
            agents: self
                .agents
                .iter()
                .map(|a| (a.id, a.data.lock().clone()))
                .collect(),
        }
    }

    pub fn load_checkpoint(ckpt: EngineCheckpoint) -> (Self, Schedule) {
        let mut schedule = Schedule::new();
        let (tx, rx) = unbounded();

        let mut agents = Vec::with_capacity(ckpt.agents.len());
        for (id, data) in ckpt.agents {
            let agent = NodeAgent {
                id,
                data: Arc::new(Mutex::new(data)),
            };
            agents.push(agent.clone());
            schedule.schedule_repeating(Box::new(agent), 0.0, 0);
        }

        let model = PrimeTimeModel {
            step: ckpt.step,
            agents,
            net_entropy: Arc::new(AtomicI64::new(ckpt.net_entropy)),
            system_surplus: Arc::new(AtomicI64::new(ckpt.system_surplus)),
            void_events: Arc::new(AtomicI64::new(ckpt.void_events)),
            surplus_events: Arc::new(AtomicI64::new(ckpt.surplus_events)),
            total_wealth: Arc::new(AtomicU64::new(ckpt.total_wealth)),
            total_vault_books: Arc::new(AtomicU64::new(ckpt.total_vault_books)),
            books_standard: Arc::new(AtomicU64::new(ckpt.books_standard)),
            books_heuristic: Arc::new(AtomicU64::new(ckpt.books_heuristic)),

            transfer_tx: tx,
            transfer_rx: rx,
        };

        (model, schedule)
    }
}
