use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct StatsProps {
    pub vault_books: f64,
    pub surplus: f64,
    pub total_scarcity: f64,
    pub entropy: f64,
}

#[function_component(Stats)]
pub fn stats(props: &StatsProps) -> Html {
    html! {
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
             <div class="glass-panel p-4 rounded-xl space-y-1 border-l-2 border-cyan-500">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Total Scarcity" }</p>
                <p class="text-lg font-bold text-white font-mono">{ format!("{:.0}", props.total_scarcity) }</p>
            </div>
            <div class="glass-panel p-4 rounded-xl space-y-1 border-l-2 border-emerald-500">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Cold Supply" }</p>
                <div class="flex items-baseline gap-2">
                    <p class="text-lg font-bold text-emerald-400 font-mono">{ format!("{:.4}", props.vault_books) }</p>
                    <span class="text-xs text-emerald-500 font-mono" title="System Surplus">{ format!("(+{:.0})", props.surplus) }</span>
                </div>
            </div>
             <div class="glass-panel p-4 rounded-xl space-y-1 border-l-2 border-orange-500">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Active Nodes" }</p>
                <p class="text-lg font-bold text-orange-400 font-mono">{ "1" }</p>
            </div>
            <div class="glass-panel p-4 rounded-xl space-y-1">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Net Entropy" }</p>
                <p class="text-lg font-bold text-white font-mono">{ format!("{:.0}", props.entropy) }</p>
            </div>
        </div>
    }
}
