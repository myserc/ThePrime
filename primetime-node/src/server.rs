use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State as AxumState},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tokio::sync::broadcast;
use serde::Serialize;
use std::sync::Arc;
use std::net::SocketAddr;

#[derive(Serialize, Clone, Debug)]
pub struct SimUpdate {
    pub tick: u64,
    pub net_entropy: i64,
    pub void_events: i64,
    pub surplus_events: i64,
    pub peer_count: usize,
    pub local_vault_books: u64,
    pub local_prime_value: u64,
    pub hash_rate: u64, // Iterations per second
}

pub async fn start_server(tx: broadcast::Sender<String>, port: u16) {
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/ws", get(ws_handler))
        .with_state(tx);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Starting UI Server on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, AxumState(tx): AxumState<broadcast::Sender<String>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

async fn handle_socket(mut socket: WebSocket, tx: broadcast::Sender<String>) {
    let mut rx = tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() { break; }
    }
}

async fn serve_dashboard() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>PrimeTime Decentralized Node</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body { background-color: #020617; color: white; font-family: sans-serif; text-align: center; }
        .dashboard { display: flex; flex-direction: column; align-items: center; margin-top: 20px; }
        .chart-container { width: 800px; height: 400px; background-color: #0f172a; border: 1px solid #334155; border-radius: 8px; padding: 10px; }
        .stats { margin: 20px; font-size: 1.2rem; font-weight: bold; }
        .telemetry-row { margin-bottom: 8px; font-size: 1.1rem; font-weight: bold; }
    </style>
</head>
<body>
    <h2>Alpha Book Protocol: Node Explorer</h2>
    <div class="stats">
        Tick: <span id="tick">0</span> |
        Connected Peers: <span id="peer_count" style="color:#22d3ee;">0</span> |
        Hash Rate: <span id="hash_rate" style="color:#fcd34d;">0</span> iters/s
    </div>
    <div class="stats">
        Voids: <span id="voids" style="color:#fb7185;">0</span> |
        Surplus: <span id="surplus" style="color:#34d399;">0</span> |
        Net Entropy: <span id="entropy_val">0</span>
    </div>

    <div class="dashboard">
        <div style="font-size: 1.5rem; font-weight: bold; color: #fcd34d;">
            Local Vault Books: <span id="vault_books">0</span>
        </div>
        <div style="font-size: 1.1rem; color: #94a3b8; margin-bottom: 15px;">
            Prime Value: <span id="prime_value">0</span>
        </div>

        <div class="chart-container">
            <canvas id="entropyChart"></canvas>
        </div>
    </div>

    <script>
        const ctx = document.getElementById('entropyChart').getContext('2d');
        const TOTAL_BOOK_COUNTS = 648000;
        const chart = new Chart(ctx, {
            type: 'line',
            data: {
                labels: [], datasets: [
                    { label: 'Global Net Entropy', borderColor: '#22d3ee', data: [], tension: 0.1 },
                    { label: '+ Limit', borderColor: '#34d399', borderDash: [5, 5], data: [], pointRadius: 0, borderWidth: 1 },
                    { label: '- Limit', borderColor: '#fb7185', borderDash: [5, 5], data: [], pointRadius: 0, borderWidth: 1 }
                ]
            },
            options: { responsive: true, maintainAspectRatio: false, animation: false, scales: { x: { display: false } } }
        });

        const ws = new WebSocket("ws://" + window.location.host + "/ws");
        ws.onmessage = function(event) {
            const data = JSON.parse(event.data);

            document.getElementById('tick').innerText = data.tick.toLocaleString();
            document.getElementById('peer_count').innerText = data.peer_count;
            document.getElementById('hash_rate').innerText = data.hash_rate.toLocaleString();
            document.getElementById('voids').innerText = data.void_events.toLocaleString();
            document.getElementById('surplus').innerText = data.surplus_events.toLocaleString();
            document.getElementById('entropy_val').innerText = data.net_entropy.toLocaleString();
            document.getElementById('vault_books').innerText = data.local_vault_books.toLocaleString();
            document.getElementById('prime_value').innerText = data.local_prime_value.toLocaleString();

            chart.data.labels.push(data.tick);
            chart.data.datasets[0].data.push(data.net_entropy);
            chart.data.datasets[1].data.push(TOTAL_BOOK_COUNTS);
            chart.data.datasets[2].data.push(-TOTAL_BOOK_COUNTS);

            if(chart.data.labels.length > 100) {
                chart.data.labels.shift();
                chart.data.datasets.forEach(ds => ds.data.shift());
            }
            chart.update();
        };
    </script>
</body>
</html>
    "#)
}
