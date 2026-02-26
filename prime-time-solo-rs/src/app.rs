use yew::prelude::*;
use gloo_timers::callback::Interval;
use gloo_utils::format::JsValueSerdeExt;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::MessageEvent;
use crate::models::{Participant, Config, ActiveBook, Book, UNITS};
use crate::engine::Engine;
use crate::persistence::{AppState, load};
use crate::components::node::Node;
use crate::components::stats::Stats;
use crate::components::ledger::Ledger;
use crate::components::footer::Footer;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Tick,
    TogglePause,
    Deposit,
    SetTab(String),
}

pub struct App {
    state: AppState,
    engine: Engine,
    config: Config,
    interval: Option<Interval>,
    active_tab: String,
    is_debug: bool,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let config = Config::default();
        let mut engine = Engine::new();
        engine.init(&config);

        // Load or default
        let mut state = load().unwrap_or_else(|| AppState {
            participant: None,
            books: vec![],
            total_scarcity: 0.0,
            net_entropy: 0.0,
            sim_time: 0.0,
            system_surplus: 0.0,
        });

        // Initialize genesis if empty
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

        let link = ctx.link().clone();
        // 100ms interval for smoother UI updates, but logical tick adjusted for 2s per count
        let interval = Interval::new(100, move || link.send_message(Msg::Tick));

        Self {
            state,
            engine,
            config,
            interval: Some(interval),
            active_tab: "books".to_string(),
            is_debug: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => {
                // Ticking Rate: 2 seconds per count.
                // Interval is 100ms.
                // 2000ms = 1 count.
                // 100ms = 1/20 = 0.05 counts.
                let steps = 0.05;

                if let Some(p) = &mut self.state.participant {
                    let threshold = self.engine.standard_mint_scarcity as f64;

                    if (p.prime_value as f64) < threshold {
                        // Not ready to mint yet

                        // Burn logic
                        if p.active_book.is_none() && p.vault_books > 0.0 {
                            p.vault_books -= 1.0;
                            p.active_book = Some(ActiveBook {
                                remaining_counts: self.config.total_book_counts as f64,
                                max_counts: self.config.total_book_counts as f64,
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
                         // Minting logic
                         p.vault_books += 1.0;
                         // Rolling momentum:
                         // p.counts -= cost;
                         p.is_online = true;

                         // Log book
                         self.state.books.insert(0, Book {
                             id: js_sys::Date::now(),
                             owner: p.name.clone(),
                             value: 1.0,
                             type_: "STANDARD".to_string(),
                             time: "Now".to_string(), // Simplified time
                         });

                         // Reset counts partially? Original logic kept counts but reset prime value calculation base.
                         // "p.counts = Math.max(0, p.counts - countsConsumed);"
                         // For simplicity in solo mode, let's just subtract threshold counts.
                         let consumed = self.config.total_book_counts as f64; // Standard mint cost
                         p.counts = (p.counts - consumed).max(0.0);
                    }

                    // Recalculate Prime Value
                    let ordinal = if p.counts > 1.0 { p.counts as usize - 1 } else { 0 };
                    p.prime_value = self.engine.get_prime_at(ordinal);

                    self.state.total_scarcity = p.prime_value as f64;
                }

                self.state.sim_time += 100.0;
                true
            }
            Msg::Deposit => {
                if let Some(p) = &mut self.state.participant {
                    p.vault_books += 1.0;
                }
                true
            }
            Msg::TogglePause => {
                if self.interval.is_some() {
                    self.interval = None;
                } else {
                    let link = ctx.link().clone();
                    self.interval = Some(Interval::new(100, move || link.send_message(Msg::Tick)));
                }
                true
            }
            Msg::SetTab(tab) => {
                self.active_tab = tab;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let p = self.state.participant.as_ref().unwrap();

        html! {
            <div class="min-h-screen flex flex-col p-4 md:p-6 pb-24"> // Added pb-24 for footer
                // Header
                <header class="flex flex-col md:flex-row justify-between items-start md:items-center gap-4 border-b border-slate-800 pb-6 mb-6">
                    <div>
                        <h1 class="text-3xl font-black tracking-tighter text-white flex items-center gap-3">
                            { "PRIME-TIME" }
                            <span class="text-xs bg-cyan-500-30 text-cyan-400 px-2 py-1 rounded border border-cyan-500-30 font-mono">{ "SOLO ENGINE" }</span>
                        </h1>
                        <div class="flex items-center gap-4 mt-2">
                             <p class="text-xs text-slate-500 tracking-widest uppercase font-bold">{ "Individualized Simulation" }</p>
                             <span class="w-1 h-1 bg-slate-700 rounded-full"></span>
                             <p class="text-xs text-slate-500 tracking-widest uppercase">{ format!("TICK: {:.0}", self.state.sim_time / 1000.0) }</p>
                        </div>
                    </div>
                    <div class="flex flex-wrap gap-2 items-center">
                        <button onclick={ctx.link().callback(|_| Msg::TogglePause)} class="btn-control">{ if self.interval.is_some() { "RUNNING" } else { "PAUSED" } }</button>
                    </div>
                </header>

                // Stats
                <Stats
                    vault_books={p.vault_books}
                    surplus={self.state.system_surplus}
                    total_scarcity={self.state.total_scarcity}
                    entropy={self.state.net_entropy}
                />

                // Main Layout
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 flex-1 min-h-0 mt-6">
                    // Node Center
                    <div class="lg:col-span-2 flex flex-col justify-center min-h-0">
                        <Node
                            participant={p.clone()}
                            on_deposit={ctx.link().callback(|_| Msg::Deposit)}
                        />
                    </div>

                    // Sidebar (Ledger)
                    <div class="flex flex-col gap-6 min-h-0 h-full">
                        <Ledger
                            books={self.state.books.clone()}
                        />
                    </div>
                </div>

                // Sticky Footer
                <Footer
                    precedents={self.engine.precedents.clone()}
                    current_prime_value={p.prime_value}
                />
            </div>
        }
    }
}
