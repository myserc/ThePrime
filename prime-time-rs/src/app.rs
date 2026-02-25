use yew::prelude::*;
use gloo_timers::callback::Interval;
use gloo_utils::format::JsValueSerdeExt;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::MessageEvent; // Added import
use crate::models::{Participant, Config, ActiveBook};
use crate::engine::Engine;
use crate::persistence::{AppState, load};
use crate::components::node::Node;
use crate::components::stats::Stats;
use crate::components::ledger::Ledger;
use crate::components::modals::InitModal;

#[derive(Clone, Debug, PartialEq)] // Removed Eq
pub enum Msg {
    Tick,
    TogglePause,
    OpenInitModal,
    CloseInitModal,
    DeployNode(String, f64),
    RemoveNode(u32),
    Deposit(u32),
    SetTab(String),
    WorkerMessage(u32, JsValue), // (NodeID, Message)
}

pub struct App {
    state: AppState,
    engine: Engine,
    config: Config,
    interval: Option<Interval>,
    show_init_modal: bool,
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
            participants: vec![],
            books: vec![],
            transfers: vec![],
            total_scarcity: 0.0,
            net_entropy: 0.0,
            sim_time: 0.0,
            id_counter: 1000,
            is_auto: false,
            system_surplus: 0.0,
        });

        // Initialize genesis if empty
        if state.participants.is_empty() {
             ctx.link().send_message(Msg::DeployNode("Node_Alpha".to_string(), 2.0));
        } else {
             // Re-attach workers
             for p in state.participants.iter_mut() {
                 if let Ok(worker) = web_sys::Worker::new("ai-worker.js") {
                     let link = ctx.link().clone();
                     let id = p.id;
                     let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                         link.send_message(Msg::WorkerMessage(id, e.data()));
                     }) as Box<dyn FnMut(MessageEvent)>);
                     worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                     onmessage.forget(); // Leak to keep alive

                     // Send Init
                     let init_msg = serde_json::json!({
                         "type": "INIT",
                         "payload": { "id": id, "config": {} }
                     });
                     let _ = worker.post_message(&JsValue::from_serde(&init_msg).unwrap());

                     p.worker = Some(worker);
                 }
             }
        }

        let link = ctx.link().clone();
        let interval = Interval::new(40, move || link.send_message(Msg::Tick)); // 25fps approx

        Self {
            state,
            engine,
            config,
            interval: Some(interval),
            show_init_modal: false,
            active_tab: "books".to_string(),
            is_debug: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => {
                let steps = 1.0;
                let mut tick_total_scarcity = 0.0;

                for p in self.state.participants.iter_mut() {
                    let threshold = self.engine.standard_mint_scarcity as f64;

                    if (p.prime_value as f64) < threshold {
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
                         p.vault_books += 1.0;
                         p.is_online = true;
                    }

                    // Recalculate Prime Value
                    let ordinal = if p.counts > 1.0 { p.counts as usize - 1 } else { 0 };
                    p.prime_value = self.engine.get_prime_at(ordinal);

                    tick_total_scarcity += p.prime_value as f64;

                    // Worker Update (Periodically)
                    if let Some(worker) = &p.worker {
                         if js_sys::Math::random() < 0.05 { // Use js_sys::Math::random
                             let msg = serde_json::json!({
                                 "type": "TICK",
                                 "payload": {
                                     "id": p.id,
                                     "primeValue": p.prime_value,
                                     "participants": []
                                 }
                             });
                             let _ = worker.post_message(&JsValue::from_serde(&msg).unwrap());
                         }
                    }
                }

                self.state.total_scarcity = tick_total_scarcity;
                self.state.sim_time += 40.0;

                true
            }
            Msg::DeployNode(name, books) => {
                let id = self.state.id_counter;
                self.state.id_counter += 1;

                let mut p = Participant {
                    id,
                    name,
                    vault_books: books,
                    active_book: None,
                    counts: 0.0,
                    prime_value: 0,
                    balance_adjustment: 0,
                    active_heuristic: None,
                    last_receipt: None,
                    is_online: true,
                    joined_at: 0,
                    worker: None,
                };

                if let Ok(worker) = web_sys::Worker::new("ai-worker.js") {
                     let link = ctx.link().clone();
                     let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                         link.send_message(Msg::WorkerMessage(id, e.data()));
                     }) as Box<dyn FnMut(MessageEvent)>);
                     worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                     onmessage.forget();

                     let init_msg = serde_json::json!({
                         "type": "INIT",
                         "payload": { "id": id, "config": {} }
                     });
                     let _ = worker.post_message(&JsValue::from_serde(&init_msg).unwrap());
                     p.worker = Some(worker);
                }

                self.state.participants.push(p);
                true
            }
            Msg::RemoveNode(id) => {
                if let Some(pos) = self.state.participants.iter().position(|x| x.id == id) {
                    if let Some(worker) = &self.state.participants[pos].worker {
                        worker.terminate();
                    }
                    self.state.participants.remove(pos);
                }
                true
            }
            Msg::TogglePause => {
                if self.interval.is_some() {
                    self.interval = None;
                } else {
                    let link = ctx.link().clone();
                    self.interval = Some(Interval::new(40, move || link.send_message(Msg::Tick)));
                }
                true
            }
            Msg::OpenInitModal => {
                self.show_init_modal = true;
                true
            }
             Msg::CloseInitModal => {
                self.show_init_modal = false;
                true
            }
            Msg::SetTab(tab) => {
                self.active_tab = tab;
                true
            }
            // Handle Worker Actions
            Msg::WorkerMessage(node_id, data) => {
                 if let Ok(val) = data.into_serde::<serde_json::Value>() {
                     if let Some(type_) = val["type"].as_str() {
                         if type_ == "ACTION_TRANSFER" {
                              // Perform transfer logic
                              // Need payload
                              if let Some(payload) = val.get("payload") {
                                  if let Some(target_id) = payload["targetId"].as_u64() {
                                      // Transfer logic here.
                                      // For brevity, simplifed: just log
                                      gloo_console::log!(format!("Transfer from {} to {}", node_id, target_id));
                                  }
                              }
                         }
                     }
                 }
                 true
            }
            _ => false
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let active_nodes = self.state.participants.iter().filter(|p| p.is_online).count();

        html! {
            <div class="min-h-screen flex flex-col p-4 md:p-6">
                // Header
                <header class="flex flex-col md:flex-row justify-between items-start md:items-center gap-4 border-b border-slate-800 pb-6 mb-6">
                    <div>
                        <h1 class="text-3xl font-black tracking-tighter text-white flex items-center gap-3">
                            { "PRIME-TIME" }
                            <span class="text-xs bg-cyan-500-30 text-cyan-400 px-2 py-1 rounded border border-cyan-500-30 font-mono">{ "AUTARKIC ENGINE" }</span>
                        </h1>
                        <div class="flex items-center gap-4 mt-2">
                             <p class="text-xs text-slate-500 tracking-widest uppercase font-bold">{ "5-Tier Heuristic Liquidity Spectrum" }</p>
                             <span class="w-1 h-1 bg-slate-700 rounded-full"></span>
                             <p class="text-xs text-slate-500 tracking-widest uppercase">{ format!("TICK: {:.0}", self.state.sim_time / 1000.0) }</p>
                        </div>
                    </div>
                    <div class="flex flex-wrap gap-2 items-center">
                        <button onclick={ctx.link().callback(|_| Msg::TogglePause)} class="btn-control">{ if self.interval.is_some() { "RUNNING" } else { "PAUSED" } }</button>
                        <button onclick={ctx.link().callback(|_| Msg::OpenInitModal)} class="btn-primary">{ "+ Initialize Node" }</button>
                    </div>
                </header>

                // Stats
                <Stats
                    active_supply={0.0}
                    vault_books={self.state.participants.iter().map(|p| p.vault_books).sum::<f64>()}
                    surplus={self.state.system_surplus}
                    total_scarcity={self.state.total_scarcity}
                    active_nodes={active_nodes}
                    entropy={self.state.net_entropy}
                    net_entropy_residue={0.0}
                />

                // Main Layout
                <div class="grid grid-cols-1 xl:grid-cols-4 gap-6 flex-1 min-h-0 mt-6">
                    // Node Grid
                    <div class="xl:col-span-3 flex flex-col gap-4 min-h-0">
                         <div class="flex items-center justify-between bg-slate-900-30 p-3 rounded-lg border border-slate-800">
                            <h2 class="text-sm font-bold text-white uppercase tracking-widest">{ "Active Nodes" }</h2>
                         </div>
                         <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                            { for self.state.participants.iter().map(|p| {
                                html! {
                                    <Node
                                        participant={p.clone()}
                                        on_deposit={ctx.link().callback(|id| Msg::Deposit(id))}
                                        on_remove={ctx.link().callback(|id| Msg::RemoveNode(id))}
                                        on_transfer_drag={Callback::from(|_| {})}
                                        on_transfer_drop={Callback::from(|_| {})}
                                        transfer_source_id={None}
                                        is_debug={self.is_debug}
                                        active_heuristic={p.active_heuristic.clone()}
                                    />
                                }
                            }) }
                         </div>
                    </div>

                    // Sidebar
                    <div class="flex flex-col gap-6 min-h-0">
                        <Ledger
                            books={self.state.books.clone()}
                            transfers={self.state.transfers.clone()}
                            active_tab={self.active_tab.clone()}
                            on_tab_change={ctx.link().callback(Msg::SetTab)}
                        />
                    </div>
                </div>

                <InitModal
                    is_open={self.show_init_modal}
                    on_close={ctx.link().callback(|_| Msg::CloseInitModal)}
                    on_deploy={ctx.link().callback(|(n, b)| Msg::DeployNode(n, b))}
                />
            </div>
        }
    }
}
