use futures_util::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use serde::Deserialize;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlCanvasElement, HtmlElement};
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type UPlot;

    #[wasm_bindgen(constructor, js_namespace = window)]
    fn new(opts: &js_sys::Object, data: &js_sys::Array, el: &HtmlElement) -> UPlot;

    #[wasm_bindgen(method)]
    fn setData(this: &UPlot, data: &js_sys::Array);
}

#[derive(Deserialize, Clone, Default)]
struct SimUpdate {
    tick: u64,
    net_entropy: i64,
    void_events: i64,
    surplus_events: i64,
    total_wealth: u64,
    total_vault_books: u64,
    books_standard: u64,
    books_heuristic: u64,
    agent_deltas: Vec<i64>,
}

#[function_component(App)]
fn app() -> Html {
    let update_state = use_state(|| Rc::new(SimUpdate::default()));

    // Arrays for charting data
    let tick_history = use_mut_ref(|| Vec::<f64>::new());
    let entropy_history = use_mut_ref(|| Vec::<f64>::new());
    let vaults_history = use_mut_ref(|| Vec::<f64>::new());
    let voids_history = use_mut_ref(|| Vec::<f64>::new());

    let entropy_chart_ref = use_node_ref();
    let velocity_chart_ref = use_node_ref();

    let uplot_entropy = use_mut_ref(|| None::<UPlot>);
    let uplot_velocity = use_mut_ref(|| None::<UPlot>);

    // Initialize uPlot charts once
    {
        let entropy_chart_ref = entropy_chart_ref.clone();
        let velocity_chart_ref = velocity_chart_ref.clone();
        let uplot_entropy = uplot_entropy.clone();
        let uplot_velocity = uplot_velocity.clone();

        use_effect_with((), move |_| {
            // Setup Entropy Chart
            if let Some(el) = entropy_chart_ref.cast::<HtmlElement>() {
                let opts_json = "{\"width\": 600, \"height\": 250, \"title\": \"Net Entropy\", \"axes\": [{\"stroke\": \"#475569\"}, {\"stroke\": \"#475569\"}], \"series\": [{}, {\"stroke\": \"#22d3ee\", \"fill\": \"rgba(34, 211, 238, 0.1)\"}]}";
                let opts: js_sys::Object = js_sys::JSON::parse(opts_json).unwrap().into();
                let initial_data = js_sys::Array::of2(&js_sys::Array::new(), &js_sys::Array::new());
                *uplot_entropy.borrow_mut() = Some(UPlot::new(&opts, &initial_data, &el));
            }

            // Setup Velocity Chart
            if let Some(el) = velocity_chart_ref.cast::<HtmlElement>() {
                let opts_json = "{\"width\": 600, \"height\": 250, \"title\": \"Vaults & Voids\", \"axes\": [{\"stroke\": \"#475569\"}, {\"stroke\": \"#fcd34d\"}, {\"stroke\": \"#fb7185\", \"side\": 1}], \"series\": [{}, {\"stroke\": \"#fcd34d\", \"fill\": \"rgba(252, 211, 77, 0.1)\", \"label\": \"Vaults\"}, {\"stroke\": \"#fb7185\", \"fill\": \"rgba(251, 113, 133, 0.1)\", \"label\": \"Voids\", \"scale\": \"y1\"}], \"scales\": {\"y1\": {\"auto\": true}}}";
                let opts: js_sys::Object = js_sys::JSON::parse(opts_json).unwrap().into();
                let initial_data = js_sys::Array::of3(
                    &js_sys::Array::new(),
                    &js_sys::Array::new(),
                    &js_sys::Array::new(),
                );
                *uplot_velocity.borrow_mut() = Some(UPlot::new(&opts, &initial_data, &el));
            }
            || ()
        });
    }

    // Connect to WebSockets
    {
        let update_state = update_state.clone();
        use_effect_with((), move |_| {
            let ws = WebSocket::open("ws://127.0.0.1:3005/ws").unwrap();
            let (_, mut read) = ws.split();

            spawn_local(async move {
                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Bytes(bin) = msg {
                        if let Ok(update) = bincode::deserialize::<SimUpdate>(&bin) {
                            update_state.set(Rc::new(update));
                        }
                    }
                }
            });
            || ()
        });
    }

    let u = (*update_state).clone();

    // Canvas agent grid rendering
    let canvas_ref = use_node_ref();
    {
        let canvas_ref = canvas_ref.clone();
        let deltas = u.agent_deltas.clone();
        use_effect_with(deltas, move |deltas| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                if let Ok(Some(context)) = canvas.get_context("2d") {
                    let ctx = context
                        .dyn_into::<web_sys::CanvasRenderingContext2d>()
                        .unwrap();
                    ctx.clear_rect(0.0, 0.0, 600.0, 600.0);

                    let mut i = 0;
                    for y in 0..50 {
                        for x in 0..50 {
                            if i >= deltas.len() {
                                break;
                            }
                            let d = deltas[i];
                            if d > 0 {
                                ctx.set_fill_style_str("#34d399");
                            } else if d < 0 {
                                ctx.set_fill_style_str("#fb7185");
                            } else {
                                ctx.set_fill_style_str("#334155");
                            }
                            ctx.fill_rect(x as f64 * 12.0, y as f64 * 12.0, 11.0, 11.0);
                            i += 1;
                        }
                    }
                }
            }
            || ()
        });
    }

    // Update charts effect
    {
        let u = u.clone();
        let tick_history = tick_history.clone();
        let entropy_history = entropy_history.clone();
        let vaults_history = vaults_history.clone();
        let voids_history = voids_history.clone();
        let uplot_entropy = uplot_entropy.clone();
        let uplot_velocity = uplot_velocity.clone();

        use_effect_with(u.tick, move |&_tick| {
            if u.tick > 0 {
                let mut th = tick_history.borrow_mut();
                let mut eh = entropy_history.borrow_mut();
                let mut vh = vaults_history.borrow_mut();
                let mut vdh = voids_history.borrow_mut();

                th.push(u.tick as f64);
                eh.push(u.net_entropy as f64);
                vh.push(u.total_vault_books as f64);
                vdh.push(u.void_events as f64);

                if th.len() > 100 {
                    th.remove(0);
                    eh.remove(0);
                    vh.remove(0);
                    vdh.remove(0);
                }

                // Push to uPlot (Net Entropy)
                if let Some(plot) = &*uplot_entropy.borrow() {
                    let th_js = js_sys::Float64Array::from(th.as_slice());
                    let eh_js = js_sys::Float64Array::from(eh.as_slice());
                    let data = js_sys::Array::of2(&th_js, &eh_js);
                    plot.setData(&data);
                }

                // Push to uPlot (Velocity)
                if let Some(plot) = &*uplot_velocity.borrow() {
                    let th_js = js_sys::Float64Array::from(th.as_slice());
                    let vh_js = js_sys::Float64Array::from(vh.as_slice());
                    let vdh_js = js_sys::Float64Array::from(vdh.as_slice());
                    let data = js_sys::Array::of3(&th_js, &vh_js, &vdh_js);
                    plot.setData(&data);
                }
            }
            || ()
        });
    }

    html! {
        <div class="p-6 font-sans flex flex-col min-h-screen bg-slate-950">
            <h1 class="text-3xl font-black mb-6 text-cyan-400">{ "PRIME-TIME Engine SPA" }</h1>

            <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
                <div class="bg-slate-900 border border-slate-800 p-4 rounded-xl">
                    <p class="text-xs text-slate-500 font-bold uppercase tracking-wider">{ "Global Tick" }</p>
                    <p class="text-2xl font-mono text-white">{ u.tick }</p>
                </div>
                <div class="bg-slate-900 border border-slate-800 p-4 rounded-xl">
                    <p class="text-xs text-slate-500 font-bold uppercase tracking-wider">{ "Net Entropy" }</p>
                    <p class="text-2xl font-mono text-cyan-300">{ u.net_entropy }</p>
                </div>
                <div class="bg-slate-900 border border-slate-800 p-4 rounded-xl">
                    <p class="text-xs text-slate-500 font-bold uppercase tracking-wider">{ "Vault Books" }</p>
                    <p class="text-2xl font-mono text-amber-400">{ u.total_vault_books }</p>
                </div>
                <div class="bg-slate-900 border border-slate-800 p-4 rounded-xl flex justify-between">
                    <div>
                        <p class="text-xs text-slate-500 font-bold uppercase tracking-wider">{ "Surplus" }</p>
                        <p class="text-2xl font-mono text-emerald-400">{ u.surplus_events }</p>
                    </div>
                    <div>
                        <p class="text-xs text-slate-500 font-bold uppercase tracking-wider">{ "Voids" }</p>
                        <p class="text-2xl font-mono text-rose-500">{ u.void_events }</p>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 xl:grid-cols-3 gap-6">
                <div class="bg-slate-900 border border-slate-800 p-4 rounded-xl xl:col-span-2">
                    <h2 class="text-sm font-bold text-slate-400 uppercase tracking-widest mb-4">{ "Agent Telemetry Sub-Cluster (2500 Units)" }</h2>
                    <canvas ref={canvas_ref} width="600" height="600" class="bg-slate-950 border border-slate-800 rounded w-full max-w-[600px]"></canvas>
                </div>

                <div class="flex flex-col gap-6">
                    <div class="bg-slate-900 border border-slate-800 p-4 rounded-xl">
                        <div ref={entropy_chart_ref}></div>
                    </div>
                    <div class="bg-slate-900 border border-slate-800 p-4 rounded-xl">
                        <div ref={velocity_chart_ref}></div>
                    </div>
                </div>
            </div>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
