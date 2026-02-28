use sled::Db;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::pow::Proof;
use crate::core::{Unit, get_ordinal_for_prime, PRIMES};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventType {
    Mint {
        proof: Proof,
        heuristic: Option<Unit>,
    },
    Transfer {
        sender: String,
        receiver: String,
        amount_prime: u64,
        heuristic: Unit,
        signature: String, // Mocked for now
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: String, // Hash of the event
    pub parent_ids: Vec<String>,
    pub event_type: EventType,
    pub entropy_delta: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalState {
    pub vault_books: u64,
    pub counts: usize,
    pub prime_value: u64,
    pub balance_adjustment: u64,
    pub active_heuristic: Option<Unit>,
    pub last_hash: String,
}

pub struct Ledger {
    db: Db,
    pub net_entropy: std::sync::atomic::AtomicI64,
    pub surplus_events: std::sync::atomic::AtomicI64,
    pub void_events: std::sync::atomic::AtomicI64,
}

impl Ledger {
    pub fn new(path: &str) -> Arc<Self> {
        let db = sled::open(path).expect("Failed to open sled DB");

        let mut entropy = 0;
        if let Ok(Some(ent)) = db.get("net_entropy") {
            let ent_bytes: [u8; 8] = ent.as_ref().try_into().unwrap();
            entropy = i64::from_le_bytes(ent_bytes);
        }

        Arc::new(Ledger {
            db,
            net_entropy: std::sync::atomic::AtomicI64::new(entropy),
            surplus_events: std::sync::atomic::AtomicI64::new(0),
            void_events: std::sync::atomic::AtomicI64::new(0),
        })
    }

    pub fn save_event(&self, event: &Event) {
        let serialized = bincode::serialize(event).unwrap();
        self.db.insert(&event.id, serialized).unwrap();

        let old_entropy = self.net_entropy.fetch_add(event.entropy_delta, std::sync::atomic::Ordering::SeqCst);
        let new_entropy = old_entropy + event.entropy_delta;

        // Simple mock of thermodynamic update
        let total_book_counts = crate::core::TOTAL_BOOK_COUNTS;
        if new_entropy >= total_book_counts {
            self.net_entropy.fetch_sub(total_book_counts, std::sync::atomic::Ordering::SeqCst);
            self.surplus_events.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        } else if new_entropy <= -total_book_counts {
            self.net_entropy.fetch_add(total_book_counts, std::sync::atomic::Ordering::SeqCst);
            self.void_events.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        let ent_bytes = self.net_entropy.load(std::sync::atomic::Ordering::SeqCst).to_le_bytes();
        self.db.insert("net_entropy", &ent_bytes).unwrap();
        self.db.flush().unwrap();
    }

    pub fn get_local_state(&self, id: &str) -> LocalState {
        let key = format!("state_{}", id);
        if let Ok(Some(val)) = self.db.get(&key) {
            bincode::deserialize(&val).unwrap_or_else(|_| Self::default_state())
        } else {
            Self::default_state()
        }
    }

    pub fn save_local_state(&self, id: &str, state: &LocalState) {
        let key = format!("state_{}", id);
        let serialized = bincode::serialize(state).unwrap();
        self.db.insert(key, serialized).unwrap();
        self.db.flush().unwrap();
    }

    fn default_state() -> LocalState {
        LocalState {
            vault_books: 2,
            counts: 1, // Start with 1 count
            prime_value: 2,
            balance_adjustment: 0,
            active_heuristic: None,
            last_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(), // genesis hash
        }
    }
}

impl LocalState {
    pub fn update_prime_value(&mut self) {
        let mut ordinal_idx = if self.counts > 0 { self.counts - 1 } else { 0 };
        ordinal_idx = ordinal_idx.min(PRIMES.len() - 1);
        self.prime_value = PRIMES[ordinal_idx] + self.balance_adjustment;
    }
}
