use yew::prelude::*;
use crate::models::{Book, Transfer};

#[derive(Properties, PartialEq)]
pub struct LedgerProps {
    pub books: Vec<Book>,
    pub transfers: Vec<Transfer>,
    pub active_tab: String,
    pub on_tab_change: Callback<String>,
}

#[function_component(Ledger)]
pub fn ledger(props: &LedgerProps) -> Html {
    let on_click_books = {
        let cb = props.on_tab_change.clone();
        Callback::from(move |_| cb.emit("books".to_string()))
    };
    let on_click_transfers = {
        let cb = props.on_tab_change.clone();
        Callback::from(move |_| cb.emit("transfers".to_string()))
    };

    html! {
        <div class="glass-panel rounded-xl flex flex-col flex-1 min-h-[400px]">
            <div class="p-4 border-b border-slate-800 flex justify-between items-center">
                <h3 class="text-xs font-bold text-white uppercase tracking-widest">{ "Global Ledger" }</h3>
                <div class="flex gap-2 text-xs font-bold">
                    <button onclick={on_click_books} class={if props.active_tab == "books" { "text-cyan-400" } else { "text-slate-500 hover:text-cyan-400" }}>{ "BOOKS" }</button>
                    <span class="text-slate-700">{ "|" }</span>
                    <button onclick={on_click_transfers} class={if props.active_tab == "transfers" { "text-purple-400" } else { "text-slate-500 hover:text-purple-400" }}>{ "TRANSFERS" }</button>
                </div>
            </div>
            <div class="flex-1 overflow-y-auto p-3 space-y-2 custom-scrollbar">
                if props.active_tab == "books" {
                    { for props.books.iter().take(20).map(|book| html! {
                        <div class="flex justify-between items-center p-2 bg-slate-900-50 rounded border border-slate-800 text-xs">
                            <div>
                                <span class="text-cyan-400 font-bold block">{ &book.owner }</span>
                                <span class="text-slate-600">{ &book.time }</span>
                            </div>
                            <div class="text-right">
                                <span class="text-white font-bold block">{ format!("+{} BOOK", book.value) }</span>
                                <span class="text-xs text-slate-400">{ &book.type_ }</span>
                            </div>
                        </div>
                    }) }
                } else {
                     { for props.transfers.iter().take(20).map(|transfer| html! {
                        <div class="flex justify-between items-center p-2 bg-slate-900-50 rounded border border-slate-800 text-xs">
                            <div>
                                <div class="flex gap-1">
                                    <span class="text-slate-400">{ &transfer.from }</span>
                                    <span class="text-purple-400">{ "→" }</span>
                                    <span class="text-white font-bold">{ &transfer.to }</span>
                                </div>
                                <span class="text-slate-600">{ &transfer.time }</span>
                            </div>
                            <div class="text-right">
                                <span class="text-purple-400 font-bold block">{ format!("{}{}", transfer.amount, "°") }</span> // Simplified symbol
                                <span class="text-xs text-emerald-500">{ format!("ΔS: {}", transfer.entropy) }</span>
                            </div>
                        </div>
                    }) }
                }
            </div>
        </div>
    }
}
