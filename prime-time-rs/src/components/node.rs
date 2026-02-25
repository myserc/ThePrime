use yew::prelude::*;
use crate::models::Participant;

#[derive(Properties, PartialEq)]
pub struct NodeProps {
    pub participant: Participant,
    pub on_deposit: Callback<u32>,
    pub on_remove: Callback<u32>,
    pub on_transfer_drag: Callback<u32>,
    pub on_transfer_drop: Callback<u32>,
    pub transfer_source_id: Option<u32>,
    pub is_debug: bool,
    pub active_heuristic: Option<String>,
}

#[function_component(Node)]
pub fn node(props: &NodeProps) -> Html {
    let p = &props.participant;
    let id = p.id;
    let pct = 50.0;

    let on_deposit = props.on_deposit.clone();
    let on_remove = props.on_remove.clone();
    let on_drag_start = props.on_transfer_drag.clone();
    let on_drop = props.on_transfer_drop.clone();

    let border_class = if props.transfer_source_id == Some(id) {
        "border-cyan-400 shadow-[0_0_15px_rgba(34,211,238,0.3)]"
    } else if props.transfer_source_id.is_some() {
        "border-purple-500-30 cursor-pointer hover:border-purple-400 hover:bg-purple-500-10"
    } else if p.active_heuristic.is_some() {
        "border-purple-500-30 active-heuristic"
    } else if p.is_online {
        "border-slate-700 active-standard"
    } else {
        "border-slate-800 node-offline"
    };

    html! {
        <div
            id={format!("node-{}", id)}
            class={format!("glass-panel p-4 rounded-xl relative overflow-hidden transition-all duration-300 cursor-grab active:cursor-grabbing {}", border_class)}
            draggable="true"
            ondragstart={move |_| on_drag_start.emit(id)}
            ondragover={Callback::from(|e: DragEvent| e.prevent_default())}
            ondrop={move |e: DragEvent| { e.prevent_default(); on_drop.emit(id); }}
        >
            <div class="absolute top-0 left-0 h-1 transition-all duration-300 bg-slate-500" style={format!("width: {}%", pct)}></div>

            <div class="flex justify-between items-start mb-3 pointer-events-none">
                <div>
                    <div class="flex items-center gap-2">
                        <h4 class="text-xs font-bold text-white tracking-wider">{ &p.name }</h4>
                        if let Some(h) = &p.active_heuristic {
                           <span class="text-[8px] px-1 rounded border border-current opacity-80 text-purple-400">{ h }</span>
                        }
                    </div>
                    <div class="flex items-center gap-2 mt-1">
                        <span class={if p.is_online { "text-[8px] px-1.5 py-0.5 rounded bg-emerald-500-10 text-emerald-400" } else { "text-[8px] px-1.5 py-0.5 rounded bg-red-500-10 text-red-400" }}>
                            { if p.is_online { "MINING" } else { "OFFLINE" } }
                        </span>
                    </div>
                </div>
                <div class="flex flex-col items-end gap-1 pointer-events-auto">
                    <button onclick={move |_| on_remove.emit(id)} class="text-[8px] text-red-500 hover:text-red-400 uppercase font-bold bg-slate-900-30 px-2 py-1 rounded border border-red-500">
                        { "Remove" }
                    </button>
                </div>
            </div>

            <div class="grid grid-cols-1 gap-2 mb-3 pointer-events-none">
                <div class="bg-slate-900-50 p-2 rounded border border-slate-800 flex justify-between items-center">
                    <span class="text-[8px] text-slate-500 uppercase">{ "Scarcity Index" }</span>
                    <span class="text-sm font-bold text-white font-mono">{ p.prime_value }</span>
                </div>
            </div>

            <div class="flex flex-col gap-2 pt-2 border-t border-slate-800 pointer-events-none">
                <div class="flex justify-between text-[8px] text-slate-500 uppercase">
                    <span>{ format!("Books: {:.4}", p.vault_books) }</span>
                </div>
                 <div class="flex gap-2 pointer-events-auto">
                    <button onclick={move |_| on_deposit.emit(id)} class="w-full py-1.5 rounded bg-slate-800 text-cyan-400 hover:bg-cyan-500 border border-slate-700 hover:border-cyan-500 transition-colors font-bold text-[10px] uppercase tracking-wider flex items-center justify-center gap-2">
                        <span>{ "Inject Book" }</span>
                    </button>
                </div>
            </div>
        </div>
    }
}
