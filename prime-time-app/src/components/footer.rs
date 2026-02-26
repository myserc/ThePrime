use yew::prelude::*;
use crate::models::{UNITS};
use std::collections::HashMap;

#[derive(Properties, PartialEq)]
pub struct FooterProps {
    pub precedents: HashMap<String, u32>,
    pub current_prime_value: u32,
}

#[function_component(Footer)]
pub fn footer(props: &FooterProps) -> Html {
    html! {
        <footer class="fixed bottom-0 left-0 right-0 glass-panel border-t border-slate-800 p-4 z-50">
            <div class="max-w-1600 flex justify-center items-center gap-6 overflow-x-auto">
                <span class="text-xs text-slate-500 uppercase font-bold tracking-widest mr-4 sticky left-0 bg-slate-900-50 px-2 rounded">{ "Liquidity Spectrum" }</span>
                { for UNITS.iter().map(|u| {
                    let precedent = props.precedents.get(u.id).unwrap_or(&1);
                    let val = if *precedent > 0 { props.current_prime_value / precedent } else { 0 };
                    let is_active = val > 0;

                    html! {
                        <div class={format!("flex items-center gap-2 px-3 py-1 rounded border transition-all {}",
                            if is_active { format!("{} bg-slate-900-50 border-current", u.color_class) } else { "text-slate-700 border-transparent".to_string() }
                        )}>
                            <span class="text-lg">{ u.symbol }</span>
                            <div class="flex flex-col">
                                <span class="text-[10px] font-bold uppercase leading-none">{ u.name }</span>
                                <span class="text-[10px] font-mono leading-none">{ val }</span>
                            </div>
                        </div>
                    }
                }) }
            </div>
        </footer>
    }
}
