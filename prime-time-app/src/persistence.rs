use gloo_storage::{LocalStorage, Storage};
use crate::models::{Participant, Book};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AppState {
    pub participant: Option<Participant>,
    pub books: Vec<Book>,
    pub total_scarcity: f64,
    pub net_entropy: f64,
    pub sim_time: f64,
    pub system_surplus: f64,
}

const STORAGE_KEY: &str = "prime_time_solo_v1_state";

pub fn save(state: &AppState) {
    let _ = LocalStorage::set(STORAGE_KEY, state);
}

pub fn load() -> Option<AppState> {
    LocalStorage::get(STORAGE_KEY).ok()
}

pub fn clear() {
    LocalStorage::delete(STORAGE_KEY);
}
