use gloo_storage::{LocalStorage, Storage};
use crate::models::{Participant, Book, Transfer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub participants: Vec<Participant>,
    pub books: Vec<Book>,
    pub transfers: Vec<Transfer>,
    pub total_scarcity: f64,
    pub net_entropy: f64,
    pub sim_time: f64,
    pub id_counter: u32,
    pub is_auto: bool,
    pub system_surplus: f64,
}

const STORAGE_KEY: &str = "prime_time_v4_state_rust";

pub fn save(state: &AppState) {
    let _ = LocalStorage::set(STORAGE_KEY, state);
}

pub fn load() -> Option<AppState> {
    LocalStorage::get(STORAGE_KEY).ok()
}

pub fn clear() {
    LocalStorage::delete(STORAGE_KEY);
}
