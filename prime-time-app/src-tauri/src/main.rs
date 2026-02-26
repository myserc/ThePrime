#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Mutex;
use tauri::{State, Manager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod engine;
mod models;

use engine::Engine;
use models::{Config, Participant, Book, ActiveBook};

#[derive(Serialize, Deserialize, Clone)]
struct SimulationState {
    pub participant: Option<Participant>,
    pub books: Vec<Book>,
    pub total_scarcity: f64,
    pub net_entropy: f64,
    pub sim_time: f64,
    pub system_surplus: f64,
}

impl Default for SimulationState {
    fn default() -> Self {
        Self {
            participant: None,
            books: Vec::new(),
            total_scarcity: 0.0,
            net_entropy: 0.0,
            sim_time: 0.0,
            system_surplus: 0.0,
        }
    }
}

struct AppData {
    engine: Mutex<Engine>,
    state: Mutex<SimulationState>,
    config: Config,
}

fn get_state_path(app_handle: &tauri::AppHandle) -> std::path::PathBuf {
    app_handle.path_resolver().app_data_dir().unwrap().join("state.json")
}

#[tauri::command]
fn init_simulation(app_handle: tauri::AppHandle, app_data: State<AppData>) -> Result<SimulationState, String> {
    let mut engine = app_data.engine.lock().map_err(|_| "Failed to lock engine")?;
    let mut state = app_data.state.lock().map_err(|_| "Failed to lock state")?;

    // Check if initialized
    if engine.primes.is_empty() {
        engine.init(&app_data.config);
    }

    // Try load state
    let path = get_state_path(&app_handle);
    if path.exists() {
        if let Ok(file) = std::fs::File::open(path) {
            if let Ok(loaded_state) = serde_json::from_reader(file) {
                *state = loaded_state;
            }
        }
    }

    if state.participant.is_none() {
        state.participant = Some(Participant {
            id: 1,
            name: "Node_Alpha".to_string(),
            vault_books: 2.0,
            active_book: None,
            counts: 0.0,
            prime_value: 0,
            balance_adjustment: 0,
            active_heuristic: None,
            last_receipt: None,
            is_online: true,
            joined_at: 0,
        });
    }

    Ok(state.clone())
}

#[tauri::command]
fn tick_simulation(app_data: State<AppData>, steps: f64) -> Result<SimulationState, String> {
    let engine = app_data.engine.lock().map_err(|_| "Failed to lock engine")?;
    let mut state = app_data.state.lock().map_err(|_| "Failed to lock state")?;

    if let Some(p) = &mut state.participant {
        let threshold = engine.standard_mint_scarcity as f64;

        if (p.prime_value as f64) < threshold {
            if p.active_book.is_none() && p.vault_books > 0.0 {
                p.vault_books -= 1.0;
                p.active_book = Some(ActiveBook {
                    remaining_counts: app_data.config.total_book_counts as f64,
                    max_counts: app_data.config.total_book_counts as f64,
                });
            }

            if let Some(book) = &mut p.active_book {
                if book.remaining_counts >= steps {
                    book.remaining_counts -= steps;
                    p.counts += steps;
                    p.is_online = true;
                } else {
                    p.active_book = None;
                    p.is_online = false;
                }
            } else {
                p.is_online = false;
            }
        } else {
             p.vault_books += 1.0;
             p.is_online = true;

             state.books.insert(0, Book {
                 id: js_sys_date_now(),
                 owner: p.name.clone(),
                 value: 1.0,
                 type_: "STANDARD".to_string(),
                 time: "Now".to_string(),
             });

             let consumed = app_data.config.total_book_counts as f64;
             p.counts = (p.counts - consumed).max(0.0);
        }

        let ordinal = if p.counts > 1.0 { p.counts as usize - 1 } else { 0 };
        p.prime_value = engine.get_prime_at(ordinal);
        state.total_scarcity = p.prime_value as f64;
    }

    state.sim_time += steps * 2000.0; // Approximation based on loop logic

    Ok(state.clone())
}

#[tauri::command]
fn get_state(app_data: State<AppData>) -> Result<SimulationState, String> {
    let state = app_data.state.lock().map_err(|_| "Failed to lock state")?;
    Ok(state.clone())
}

#[tauri::command]
fn get_precedents(app_data: State<AppData>) -> Result<HashMap<String, u32>, String> {
    let engine = app_data.engine.lock().map_err(|_| "Failed to lock engine")?;
    Ok(engine.precedents.clone())
}

#[tauri::command]
fn perform_action(app_data: State<AppData>, action: String) -> Result<SimulationState, String> {
    let mut state = app_data.state.lock().map_err(|_| "Failed to lock state")?;
    if action == "deposit" {
        if let Some(p) = &mut state.participant {
            p.vault_books += 1.0;
        }
    }
    Ok(state.clone())
}

#[tauri::command]
fn save_state(app_handle: tauri::AppHandle, app_data: State<AppData>) -> Result<(), String> {
    let state = app_data.state.lock().map_err(|_| "Failed to lock state")?;
    let path = get_state_path(&app_handle);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer(file, &*state).map_err(|e| e.to_string())?;
    Ok(())
}

fn js_sys_date_now() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as f64
}

fn main() {
    tauri::Builder::default()
        .manage(AppData {
            engine: Mutex::new(Engine::new()),
            state: Mutex::new(SimulationState::default()),
            config: Config::default(),
        })
        .invoke_handler(tauri::generate_handler![init_simulation, tick_simulation, get_state, get_precedents, perform_action, save_state])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
