use yew::prelude::*;
use crate::models::{Book};

#[derive(Properties, PartialEq)]
pub struct LedgerProps {
    pub books: Vec<Book>,
}

#[function_component(Ledger)]
pub fn ledger(props: &LedgerProps) -> Html {
    html! {
        <div class="glass-panel rounded-xl flex flex-col flex-1 h-full min-h-[300px]">
            <div class="p-4 border-b border-slate-800 flex justify-between items-center">
                <h3 class="text-xs font-bold text-white uppercase tracking-widest">{ "Global Ledger" }</h3>
            </div>
            <div class="flex-1 overflow-y-auto p-3 space-y-2 custom-scrollbar">
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
            </div>
        </div>
    }
}
