pub mod core;
pub mod pow;
pub mod ledger;
pub mod p2p;
pub mod server;

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration, Instant};
use libp2p::swarm::SwarmEvent;
use libp2p::{gossipsub, kad, request_response};
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;

use crate::ledger::{Ledger, Event, EventType};
use crate::p2p::{P2PNode, P2PBehaviourEvent, HandshakeRequest, HandshakeResponse};
use crate::pow::{mine_iterations, verify_proof};
use crate::server::{SimUpdate, start_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let port = if args.len() > 1 { args[1].parse::<u16>().unwrap_or(3001) } else { 3001 };
    let swarm_port = if args.len() > 2 { args[2].parse::<u16>().unwrap_or(0) } else { 0 };

    let db_path = format!("primetime_db_{}", port);
    let ledger = Ledger::new(&db_path);

    let mut p2p_node = P2PNode::new()?;
    let addr = format!("/ip4/0.0.0.0/tcp/{}", swarm_port);
    p2p_node.swarm.listen_on(addr.parse()?)?;
    let local_peer_id = p2p_node.peer_id.to_string();
    println!("Node ID: {}", local_peer_id);

    // Provide a way to bootstrap nodes by specifying a peer to dial in arguments
    if args.len() > 3 {
        let bootnode_addr: Multiaddr = args[3].parse().expect("Invalid multiaddr");
        println!("Dialing bootnode: {}", bootnode_addr);
        p2p_node.swarm.dial(bootnode_addr.clone())?;

        if let Some(libp2p::multiaddr::Protocol::P2p(peer)) = bootnode_addr.iter().last() {
            p2p_node.swarm.behaviour_mut().kademlia.add_address(&peer, bootnode_addr);
        }
    }

    let _ = p2p_node.swarm.behaviour_mut().kademlia.bootstrap();

    let local_state = Arc::new(tokio::sync::RwLock::new(ledger.get_local_state(&local_peer_id)));

    // Simplistic tracking of out-bound handshake requests
    let mut pending_handshakes = std::collections::HashMap::new();

    let (tx, _rx) = broadcast::channel(100);
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        start_server(tx_clone, port).await;
    });

    let mut tick = 0;
    let mut ui_interval = interval(Duration::from_millis(500));
    let mut pow_iterations_total = 0;
    let mut last_pow_check = Instant::now();
    let mut hash_rate = 0;

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<Event>(100);

    let local_state_pow = local_state.clone();
    let ledger_pow = ledger.clone();
    let peer_id_pow = local_peer_id.clone();

    tokio::spawn(async move {
        loop {
            let mut state = local_state_pow.write().await;
            let pow_batch = 1000;
            let pow_proof = mine_iterations(&state.last_hash, pow_batch);

            state.last_hash = pow_proof.end_hash.clone();
            state.counts += 1;
            state.update_prime_value();

            let threshold = if let Some(h) = state.active_heuristic {
                crate::core::STANDARDS[&h].mint_scarcity
            } else {
                crate::core::STANDARD_MINT_SCARCITY
            };

            if state.prime_value >= threshold {
                let counts_consumed = if let Some(h) = state.active_heuristic {
                    crate::core::STANDARDS[&h].mint_counts
                } else {
                    crate::core::TOTAL_BOOK_COUNTS as usize
                };

                let efficiency_gain = crate::core::TOTAL_BOOK_COUNTS - counts_consumed as i64;

                let mint_event = Event {
                    id: pow_proof.end_hash.clone(),
                    parent_ids: vec![],
                    event_type: EventType::Mint {
                        proof: pow_proof,
                        heuristic: state.active_heuristic,
                    },
                    entropy_delta: efficiency_gain.max(0),
                };

                ledger_pow.save_event(&mint_event);
                let _ = event_tx.send(mint_event).await;

                state.vault_books += 1;
                state.counts = 1.max(state.counts.saturating_sub(counts_consumed));
                state.balance_adjustment = 0;
                state.active_heuristic = None;
                state.update_prime_value();

                ledger_pow.save_local_state(&peer_id_pow, &state);
            }
            drop(state);
            tokio::task::yield_now().await;
        }
    });

    let mut handshake_interval = interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = ui_interval.tick() => {
                let state_clone = local_state.read().await.clone();
                let net_entropy = ledger.net_entropy.load(std::sync::atomic::Ordering::Relaxed);
                let void_events = ledger.void_events.load(std::sync::atomic::Ordering::Relaxed);
                let surplus_events = ledger.surplus_events.load(std::sync::atomic::Ordering::Relaxed);

                let connected_peers = p2p_node.swarm.network_info().num_peers();

                let now = Instant::now();
                let elapsed = now.duration_since(last_pow_check).as_secs_f64();
                if elapsed > 1.0 {
                    hash_rate = (pow_iterations_total as f64 / elapsed) as u64;
                    pow_iterations_total = 0;
                    last_pow_check = now;
                }

                let update = SimUpdate {
                    tick,
                    net_entropy,
                    void_events,
                    surplus_events,
                    peer_count: connected_peers,
                    local_vault_books: state_clone.vault_books,
                    local_prime_value: state_clone.prime_value,
                    hash_rate,
                };

                let _ = tx.send(serde_json::to_string(&update).unwrap());
                tick += 1;
                pow_iterations_total += 1000 * 50;
            }

            _ = handshake_interval.tick() => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                if rng.gen_bool(0.1) {
                    let connected_peers: Vec<_> = p2p_node.swarm.connected_peers().cloned().collect();
                    if !connected_peers.is_empty() {
                        let target_peer = connected_peers[rng.gen_range(0..connected_peers.len())];
                        let state = local_state.read().await;

                        let affordable: Vec<crate::core::Unit> = crate::core::Unit::all().into_iter()
                            .filter(|u| state.prime_value >= crate::core::STANDARDS[u].precedent)
                            .collect();

                        if !affordable.is_empty() {
                            let unit = affordable[rng.gen_range(0..affordable.len())];
                            let std = &crate::core::STANDARDS[&unit];
                            let max_amount = state.prime_value / std.precedent;
                            let amount = rng.gen_range(1..=max_amount);

                            let req = HandshakeRequest {
                                sender: local_peer_id.clone(),
                                amount_prime: amount * std.precedent,
                                unit,
                            };

                            let req_id = p2p_node.swarm.behaviour_mut().req_resp.send_request(&target_peer, req.clone());
                            pending_handshakes.insert(req_id, req);
                            println!("Initiated heuristic handshake with {}", target_peer);
                        }
                    }
                }
            }

            Some(event) = event_rx.recv() => {
                let serialized_event = serde_json::to_vec(&event).unwrap();
                let topic = libp2p::gossipsub::IdentTopic::new("primetime-dag");
                let _ = p2p_node.swarm.behaviour_mut().gossipsub.publish(topic, serialized_event);
            }

            event = p2p_node.swarm.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(P2PBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { .. })) => {
                    },
                    SwarmEvent::Behaviour(P2PBehaviourEvent::Gossipsub(gossipsub::Event::Message { propagation_source: _, message_id: _, message })) => {
                        if let Ok(event) = serde_json::from_slice::<Event>(&message.data) {
                            match &event.event_type {
                                EventType::Mint { proof, .. } => {
                                    if verify_proof(proof) {
                                        ledger.save_event(&event);
                                    } else {
                                        println!("Rejected invalid Mint Proof from gossipsub");
                                    }
                                },
                                EventType::Transfer { .. } => {
                                    ledger.save_event(&event);
                                }
                            }
                        }
                    },
                    SwarmEvent::Behaviour(P2PBehaviourEvent::ReqResp(request_response::Event::Message { peer, message })) => {
                        match message {
                            request_response::Message::Request { request, channel, .. } => {
                                println!("Received HandshakeRequest from {}: {:?}", peer, request);
                                // Receiver logic
                                let mut state = local_state.write().await;
                                let old_counts = state.counts;

                                state.prime_value += request.amount_prime;
                                state.counts = crate::core::get_ordinal_for_prime(state.prime_value) + 1;
                                let new_target_base = crate::core::PRIMES[state.counts - 1];
                                state.balance_adjustment = state.prime_value.saturating_sub(new_target_base);
                                state.active_heuristic = Some(request.unit);

                                let target_leap = state.counts as i64 - old_counts as i64;

                                ledger.save_local_state(&local_peer_id, &state);

                                let response = HandshakeResponse {
                                    receiver: local_peer_id.clone(),
                                    accepted: true,
                                    handshake_id: format!("{}-{}", request.sender, local_peer_id),
                                };

                                let _ = p2p_node.swarm.behaviour_mut().req_resp.send_response(channel, response);

                                // The receiver also generates the entropy delta for their side, though
                                // typically we'd wait for the final signed Transfer event to confirm.
                            },
                            request_response::Message::Response { request_id, response, .. } => {
                                println!("Received HandshakeResponse from {}: {:?}", peer, response);
                                if response.accepted {
                                    if let Some(req) = pending_handshakes.remove(&request_id) {
                                        // Sender Logic - deduct balance and broadcast
                                        let mut state = local_state.write().await;
                                        let old_source_counts = state.counts;

                                        let new_source_val = state.prime_value.saturating_sub(req.amount_prime);
                                        let new_source_counts_idx = crate::core::get_ordinal_for_prime(new_source_val);
                                        let new_source_base = crate::core::PRIMES[new_source_counts_idx];

                                        state.prime_value = new_source_val;
                                        state.counts = new_source_counts_idx + 1;
                                        state.balance_adjustment = new_source_val.saturating_sub(new_source_base);

                                        let source_leap = state.counts as i64 - old_source_counts as i64;

                                        // Generate and broadcast Transfer Event
                                        use rand::Rng;
                                        let mut rng = rand::thread_rng();
                                        let event_id: [u8; 32] = rng.gen();

                                        let transfer_event = Event {
                                            id: hex::encode(event_id),
                                            parent_ids: vec![state.last_hash.clone()], // In reality, DAG parents
                                            event_type: EventType::Transfer {
                                                sender: local_peer_id.clone(),
                                                receiver: peer.to_string(),
                                                amount_prime: req.amount_prime,
                                                heuristic: req.unit,
                                                signature: "mock_signature".to_string(),
                                            },
                                            entropy_delta: source_leap, // the sender's portion
                                        };

                                        ledger.save_event(&transfer_event);

                                        let serialized_event = serde_json::to_vec(&transfer_event).unwrap();
                                        let topic = libp2p::gossipsub::IdentTopic::new("primetime-dag");
                                        let _ = p2p_node.swarm.behaviour_mut().gossipsub.publish(topic, serialized_event);

                                        ledger.save_local_state(&local_peer_id, &state);
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}
