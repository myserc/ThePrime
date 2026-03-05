mod p2p;
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use bincode;
use engine::{EngineCheckpoint, PrimeTimeModel, TransferIntent};
use futures_util::StreamExt;
use krabmaga::engine::state::State as KrabmagaState;
use libp2p::{Multiaddr, gossipsub, swarm::SwarmEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

// Configuration Management
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};

// Metrics
use metrics::{counter, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;

#[derive(Deserialize, Debug)]
struct Config {
    port: u16,
    agents_count: u32,
    checkpoint_interval: u64,
    metrics_port: u16,
    p2p_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 3005,
            agents_count: 2500,
            checkpoint_interval: 100,
            metrics_port: 9000,
            p2p_port: 0,
        }
    }
}

#[derive(Serialize, Clone)]
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

type BroadcastTx = broadcast::Sender<Arc<[u8]>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("PRIMETIME_"))
        .extract()
        .unwrap_or_default();

    tracing::info!("🔧 Loaded Configuration: {:?}", config);
    tracing::info!(
        "🚀 Igniting High-Capacity WebUI Server on http://0.0.0.0:{} ...",
        config.port
    );

    let builder = PrometheusBuilder::new();
    builder
        .with_http_listener(([0, 0, 0, 0], config.metrics_port))
        .install()
        .expect("failed to install Prometheus recorder");
    tracing::info!(
        "📊 Prometheus Metrics exposed on http://0.0.0.0:{}/metrics",
        config.metrics_port
    );

    // Setup P2P Swarm
    let mut swarm = p2p::create_swarm().unwrap();
    let p2p_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.p2p_port)
        .parse()
        .unwrap();
    swarm.listen_on(p2p_addr).unwrap();

    // Subscribe to Gossipsub topic
    let topic = gossipsub::IdentTopic::new("heuristic-transfers");
    let _ = swarm.behaviour_mut().gossipsub.subscribe(&topic);

    tracing::info!("🌐 P2P Engine Node listening for gossip transfers.");

    let (tx, _rx) = broadcast::channel::<Arc<[u8]>>(100);
    let tx_clone = tx.clone();

    let db = sled::open("sim_state").unwrap();
    let db_clone = db.clone(); // Clone for the checkpointing tokio task

    let loaded_checkpoint = db
        .get("checkpoint")
        .unwrap_or(None)
        .and_then(|data| bincode::deserialize::<EngineCheckpoint>(&data).ok());

    let agents_count = config.agents_count;
    let ckpt_interval = config.checkpoint_interval;

    let (mut model, mut schedule) = if let Some(ckpt) = loaded_checkpoint {
        tracing::info!(
            "♻️ Loaded checkpoint from sled DB! Resuming at tick {}",
            ckpt.step
        );
        PrimeTimeModel::load_checkpoint(ckpt)
    } else {
        tracing::info!("🌱 No checkpoint found, generating new Genesis model.");
        PrimeTimeModel::new(agents_count)
    };

    // We clone the sender so the P2P network can inject intents into the local engine
    let engine_tx = model.transfer_tx.clone();

    // Channel for the engine loop to send out intents to the P2P swarm publisher loop
    let (p2p_pub_tx, mut p2p_pub_rx) = mpsc::channel::<TransferIntent>(1000);

    // Checkpoint channel
    let (ckpt_tx, mut ckpt_rx) = mpsc::channel::<EngineCheckpoint>(2);

    // Checkpointing background task to decouple IO writes from the simulation loop
    tokio::spawn(async move {
        while let Some(ckpt) = ckpt_rx.recv().await {
            if let Ok(bin) = bincode::serialize(&ckpt) {
                let _ = db_clone.insert("checkpoint", bin);
                // Flush is synchronous and heavy, so we run it in a spawn_blocking to not block async threads
                let db_c = db_clone.clone();
                let _ = tokio::task::spawn_blocking(move || db_c.flush()).await;
                tracing::debug!("💾 Checkpoint written to disk for tick {}", ckpt.step);
            }
        }
    });

    tokio::task::spawn_blocking(move || {
        loop {
            let tick = model.step;

            for agent in model.agents.iter() {
                agent.data.lock().entropy_delta = 0;
            }

            schedule.step(&mut model);

            // Randomly route 5% of local transfer intents out to the P2P network
            // instead of resolving them locally
            if tick % 5 == 0 {
                let mut outgoing_intents = Vec::new();
                while let Ok(intent) = model.transfer_rx.try_recv() {
                    if rand::random::<f64>() < 0.05 {
                        // 5% chance to export to P2P network
                        let _ = p2p_pub_tx.blocking_send(intent.clone());
                    } else {
                        outgoing_intents.push(intent);
                    }
                }
                // Repopulate local ones
                for intent in outgoing_intents {
                    let _ = model.transfer_tx.send(intent);
                }
            }

            model.update(tick);

            let current_entropy = model.net_entropy.load(Ordering::Relaxed);
            let voids = model.void_events.load(Ordering::Relaxed);
            let surplus = model.surplus_events.load(Ordering::Relaxed);
            let vaults = model.total_vault_books.load(Ordering::Relaxed);

            gauge!("primetime_net_entropy").set(current_entropy as f64);
            gauge!("primetime_total_vaults").set(vaults as f64);
            counter!("primetime_void_events_total").absolute(voids as u64);
            counter!("primetime_surplus_events_total").absolute(surplus as u64);
            counter!("primetime_ticks_total").absolute(tick);

            if tick % 10 == 0 {
                let agent_deltas: Vec<i64> = model
                    .agents
                    .iter()
                    .take(2500)
                    .map(|a| a.data.lock().entropy_delta)
                    .collect();

                let update = SimUpdate {
                    tick,
                    net_entropy: current_entropy,
                    void_events: voids,
                    surplus_events: surplus,
                    total_wealth: model.total_wealth.load(Ordering::Relaxed),
                    total_vault_books: vaults,
                    books_standard: model.books_standard.load(Ordering::Relaxed),
                    books_heuristic: model.books_heuristic.load(Ordering::Relaxed),
                    agent_deltas,
                };

                if let Ok(encoded) = bincode::serialize(&update) {
                    let _ = tx_clone.send(Arc::from(encoded.clone()));

                    if tick > 0 && tick % ckpt_interval == 0 {
                        let ckpt = model.save_checkpoint();
                        let _ = ckpt_tx.blocking_send(ckpt);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    // Async task to handle P2P gossip
    let topic_clone = topic.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(intent) = p2p_pub_rx.recv() => {
                    // Publish the transfer intent out to the P2P network
                    if let Ok(encoded) = bincode::serialize(&intent) {
                        let _ = swarm.behaviour_mut().gossipsub.publish(topic_clone.clone(), encoded);
                    }
                }
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!("P2P Local node is listening on {}", address)
                        }
                        SwarmEvent::Behaviour(p2p::EngineBehaviourEvent::Gossipsub(
                            gossipsub::Event::Message {
                                propagation_source: peer_id,
                                message_id: id,
                                message,
                            },
                        )) => {
                            tracing::debug!(
                                "Got p2p heuristic transfer msg: '{:?}' with id: {id} from peer: {peer_id}",
                                message.data
                            );
                            // Deserialize the P2P transfer intent and route it into the engine
                            if let Ok(intent) = bincode::deserialize::<TransferIntent>(&message.data) {
                                let _ = engine_tx.send(intent);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    let app = Router::new().route("/ws", get(ws_handler)).with_state(tx);

    let bind_addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(tx): State<BroadcastTx>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

async fn handle_socket(mut socket: WebSocket, tx: BroadcastTx) {
    let mut rx = tx.subscribe();
    while let Ok(msg) = rx.recv().await {
        if socket
            .send(Message::Binary(msg.to_vec().into()))
            .await
            .is_err()
        {
            break;
        }
    }
}
