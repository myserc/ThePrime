use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct StatsProps {
    pub active_supply: f64,
    pub vault_books: f64,
    pub surplus: f64,
    pub total_scarcity: f64,
    pub active_nodes: usize,
    pub entropy: f64,
    pub net_entropy_residue: f64, // percentage
}

#[function_component(Stats)]
pub fn stats(props: &StatsProps) -> Html {
    html! {
        <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
             <div class="glass-panel p-4 rounded-xl space-y-1 border-l-2 border-cyan-500">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Active Supply" }</p>
                <p class="text-lg font-bold text-white font-mono">{ format!("{:.4}", props.active_supply) }</p>
                <p class="text-xs text-cyan-500">{ "Burning Fractions" }</p>
            </div>
            <div class="glass-panel p-4 rounded-xl space-y-1 border-l-2 border-emerald-500">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Cold Supply" }</p>
                <div class="flex items-baseline gap-2">
                    <p class="text-lg font-bold text-emerald-400 font-mono">{ format!("{:.4}", props.vault_books) }</p>
                    <span class="text-xs text-emerald-500 font-mono" title="System Surplus">{ format!("(+{:.0})", props.surplus) }</span>
                </div>
                <p class="text-xs text-emerald-500">{ "Vaulted + Surplus" }</p>
            </div>
             <div class="glass-panel p-4 rounded-xl space-y-1">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Total Scarcity" }</p>
                <p class="text-lg font-bold text-white font-mono">{ format!("{:.0}", props.total_scarcity) }</p>
                <p class="text-xs text-slate-500">{ "Σ System Value" }</p>
            </div>
             <div class="glass-panel p-4 rounded-xl space-y-1 border-l-2 border-purple-500">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Unit Values" }</p>
                 <div class="grid grid-cols-2 gap-x-2 gap-y-1 text-xs font-mono text-slate-400 mt-1">
                    // Ideally pass unit values as props, simplified here
                    <div><span class="text-rose-400">{ "◴" }</span> { "QUADRANT" }</div>
                    <div><span class="text-yellow-400">{ "☼" }</span> { "DAY" }</div>
                </div>
            </div>
            <div class="glass-panel p-4 rounded-xl space-y-1 border-l-2 border-orange-500">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Active Nodes" }</p>
                <p class="text-lg font-bold text-orange-400 font-mono">{ props.active_nodes }</p>
                <p class="text-xs text-orange-500">{ "Staking Participants" }</p>
            </div>
            <div class="glass-panel p-4 rounded-xl space-y-1">
                <p class="text-xs text-slate-500 uppercase font-bold tracking-wider">{ "Net Entropy" }</p>
                <div class="flex justify-between items-end">
                    <p class="text-lg font-bold text-white font-mono">{ format!("{:.0}", props.entropy) }</p>
                    <div class="w-12 h-1 bg-slate-800 rounded-full overflow-hidden mb-2" title="Entropy Event Progress">
                        <div class="h-full bg-cyan-400 transition-all duration-300" style={format!("width: {}%", props.net_entropy_residue)}></div>
                    </div>
                </div>
                <p class="text-xs text-slate-500">{ "System Work Delta" }</p>
            </div>
        </div>
    }
}
