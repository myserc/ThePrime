use yew::prelude::*;
use gloo_timers::callback::Interval;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::models::{Config};
use crate::persistence::AppState;
use crate::components::node::Node;
use crate::components::stats::Stats;
use crate::components::ledger::Ledger;
use crate::components::footer::Footer;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct TickArgs {
    steps: f64,
}

#[derive(Serialize, Deserialize)]
struct ActionArgs {
    action: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Tick,
    AutoSave,
    UpdateState(AppState),
    UpdatePrecedents(HashMap<String, u32>),
    TogglePause,
    Deposit,
    SetTab(String),
}

pub struct App {
    state: AppState,
    config: Config,
    interval: Option<Interval>,
    save_interval: Option<Interval>,
    active_tab: String,
    precedents: HashMap<String, u32>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();

        spawn_local(async move {
            let state_val = invoke("init_simulation", JsValue::NULL).await;
            if let Ok(state) = serde_wasm_bindgen::from_value(state_val) {
                link.send_message(Msg::UpdateState(state));
            }

            let prec_val = invoke("get_precedents", JsValue::NULL).await;
             if let Ok(prec) = serde_wasm_bindgen::from_value(prec_val) {
                link.send_message(Msg::UpdatePrecedents(prec));
            }
        });

        // Interval
        let link = ctx.link().clone();
        let interval = Interval::new(100, move || link.send_message(Msg::Tick));

        // Auto Save Interval (Every 5 seconds)
        let link_save = ctx.link().clone();
        let save_interval = Interval::new(5000, move || link_save.send_message(Msg::AutoSave));

        Self {
            state: AppState {
                participant: None,
                books: vec![],
                total_scarcity: 0.0,
                net_entropy: 0.0,
                sim_time: 0.0,
                system_surplus: 0.0,
            },
            config: Config::default(),
            interval: Some(interval),
            save_interval: Some(save_interval),
            active_tab: "books".to_string(),
            precedents: HashMap::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => {
                let steps = 0.05;
                let args = serde_wasm_bindgen::to_value(&TickArgs { steps }).unwrap();
                let link = ctx.link().clone();

                spawn_local(async move {
                    let new_state_val = invoke("tick_simulation", args).await;
                    if let Ok(state) = serde_wasm_bindgen::from_value(new_state_val) {
                        link.send_message(Msg::UpdateState(state));
                    }
                });
                false
            }
            Msg::AutoSave => {
                spawn_local(async move {
                    let _ = invoke("save_state", JsValue::NULL).await;
                });
                false
            }
            Msg::UpdateState(new_state) => {
                self.state = new_state;
                true
            }
            Msg::UpdatePrecedents(prec) => {
                self.precedents = prec;
                true
            }
            Msg::Deposit => {
                let args = serde_wasm_bindgen::to_value(&ActionArgs { action: "deposit".to_string() }).unwrap();
                let link = ctx.link().clone();
                 spawn_local(async move {
                    let new_state_val = invoke("perform_action", args).await;
                    if let Ok(state) = serde_wasm_bindgen::from_value(new_state_val) {
                        link.send_message(Msg::UpdateState(state));
                    }
                });
                false
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
        if let Some(p) = &self.state.participant {
             html! {
                <div class="min-h-screen flex flex-col p-4 md:p-6 pb-24">
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

                    <Stats
                        vault_books={p.vault_books}
                        surplus={self.state.system_surplus}
                        total_scarcity={self.state.total_scarcity}
                        entropy={self.state.net_entropy}
                    />

                    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 flex-1 min-h-0 mt-6">
                        <div class="lg:col-span-2 flex flex-col justify-center min-h-0">
                            <Node
                                participant={p.clone()}
                                on_deposit={ctx.link().callback(|_| Msg::Deposit)}
                            />
                        </div>

                        <div class="flex flex-col gap-6 min-h-0 h-full">
                            <Ledger
                                books={self.state.books.clone()}
                            />
                        </div>
                    </div>

                    <Footer
                        precedents={self.precedents.clone()}
                        current_prime_value={p.prime_value}
                    />
                </div>
            }
        } else {
            html! {
                <div class="flex items-center justify-center min-h-screen text-cyan-400 font-mono">
                    { "INITIALIZING..." }
                </div>
            }
        }
    }
}
