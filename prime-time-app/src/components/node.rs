use yew::prelude::*;
use crate::models::{Participant};

#[derive(Properties, PartialEq)]
pub struct NodeProps {
    pub participant: Participant,
    pub on_deposit: Callback<()>,
}

#[function_component(Node)]
pub fn node(props: &NodeProps) -> Html {
    let p = &props.participant;
    let pct = 50.0;

    let on_deposit = props.on_deposit.clone();

    let border_class = if p.is_online {
        "border-cyan-400 active-standard"
    } else {
        "border-slate-800 node-offline"
    };

    html! {
        <div class={format!("glass-panel p-6 rounded-xl relative overflow-hidden transition-all duration-300 w-full max-w-2xl mx-auto {}", border_class)}>
            <div class="absolute top-0 left-0 h-1 transition-all duration-300 bg-slate-500" style={format!("width: {}%", pct)}></div>

            <div class="flex justify-between items-start mb-6">
                <div>
                    <div class="flex items-center gap-2">
                        <h4 class="text-xl font-bold text-white tracking-wider">{ &p.name }</h4>
                    </div>
                    <div class="flex items-center gap-2 mt-1">
                        <span class={if p.is_online { "text-xs px-2 py-1 rounded bg-emerald-500-10 text-emerald-400" } else { "text-xs px-2 py-1 rounded bg-red-500-10 text-red-400" }}>
                            { if p.is_online { "MINING" } else { "OFFLINE" } }
                        </span>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 gap-4 mb-6">
                <div class="bg-slate-900-50 p-4 rounded border border-slate-800 flex justify-between items-center">
                    <span class="text-xs text-slate-500 uppercase">{ "Scarcity Index" }</span>
                    <span class="text-2xl font-bold text-white font-mono">{ p.prime_value }</span>
                </div>
            </div>

            <div class="flex flex-col gap-4 pt-4 border-t border-slate-800">
                <div class="flex justify-between text-xs text-slate-500 uppercase">
                    <span>{ format!("Books: {:.4}", p.vault_books) }</span>
                </div>
                 <div class="flex gap-2">
                    <button onclick={move |_| on_deposit.emit(())} class="w-full py-3 rounded bg-slate-800 text-cyan-400 hover:bg-cyan-500 border border-slate-700 hover:border-cyan-500 transition-colors font-bold text-sm uppercase tracking-wider flex items-center justify-center gap-2">
                        <span>{ "Inject Book" }</span>
                    </button>
                </div>
            </div>
        </div>
    }
}
