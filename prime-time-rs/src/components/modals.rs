use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct InitModalProps {
    pub is_open: bool,
    pub on_close: Callback<()>,
    pub on_deploy: Callback<(String, f64)>,
}

#[function_component(InitModal)]
pub fn init_modal(props: &InitModalProps) -> Html {
    let name_ref = use_node_ref();
    let books_ref = use_node_ref();

    let on_deploy = {
        let name_ref = name_ref.clone();
        let books_ref = books_ref.clone();
        let cb = props.on_deploy.clone();
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            let name = name_ref.cast::<web_sys::HtmlInputElement>().map(|i| i.value()).unwrap_or("Node_Delta".to_string());
            let books = books_ref.cast::<web_sys::HtmlInputElement>().map(|i| i.value().parse().unwrap_or(2.0)).unwrap_or(2.0);
            cb.emit((name, books));
            on_close.emit(());
        })
    };

    if !props.is_open {
        return html! {};
    }

    html! {
        <div class="modal-overlay">
            <div class="glass-panel w-full max-w-md p-6 rounded-2xl border-cyan-500-30">
                <h2 class="text-xl font-bold text-white mb-1 uppercase tracking-tighter">{ "Initialize Node" }</h2>
                <p class="text-xs text-slate-500 mb-6 uppercase">{ "Deploy a new autarkic worker" }</p>

                <div class="space-y-4 mb-6">
                    <div>
                        <label class="text-xs font-bold text-cyan-400 mb-2 uppercase tracking-wider block">{ "Node Designation" }</label>
                        <input ref={name_ref} type="text" placeholder="e.g. Node_Delta" class="w-full bg-slate-900-50 border border-slate-700 rounded px-4 py-2.5 text-xs text-white outline-none uppercase font-mono placeholder-slate-600 transition-colors" />
                    </div>
                    <div>
                        <label class="text-xs font-bold text-cyan-400 mb-2 uppercase tracking-wider block">{ "Initial Vault Balance (Books)" }</label>
                        <input ref={books_ref} type="number" value="2" min="1" max="100" class="w-full bg-slate-900-50 border border-slate-700 rounded px-4 py-2.5 text-xs text-white outline-none uppercase font-mono transition-colors" />
                    </div>
                </div>

                <div class="flex gap-2">
                    <button onclick={props.on_close.clone().reform(|_| ())} class="flex-1 px-4 py-3 rounded-lg bg-slate-800 text-xs font-bold text-slate-400 hover:text-white transition-all uppercase tracking-wider">{ "CANCEL" }</button>
                    <button onclick={on_deploy} class="flex-1 px-4 py-3 rounded-lg bg-cyan-500 text-black text-xs font-bold transition-all shadow-lg hover:bg-cyan-400 uppercase tracking-wider">{ "DEPLOY" }</button>
                </div>
            </div>
        </div>
    }
}
