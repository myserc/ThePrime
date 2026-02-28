use libp2p::{
    gossipsub, kad, noise, swarm::NetworkBehaviour, tcp, yamux, PeerId, Swarm,
    request_response,
};
use libp2p::kad::store::MemoryStore;
use libp2p::identity::Keypair;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::core::Unit;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "P2PBehaviourEvent")]
pub struct P2PBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub req_resp: request_response::cbor::Behaviour<HandshakeRequest, HandshakeResponse>,
}

#[derive(Debug)]
pub enum P2PBehaviourEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    ReqResp(request_response::Event<HandshakeRequest, HandshakeResponse>),
}

impl From<gossipsub::Event> for P2PBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        P2PBehaviourEvent::Gossipsub(event)
    }
}

impl From<kad::Event> for P2PBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        P2PBehaviourEvent::Kademlia(event)
    }
}

impl From<request_response::Event<HandshakeRequest, HandshakeResponse>> for P2PBehaviourEvent {
    fn from(event: request_response::Event<HandshakeRequest, HandshakeResponse>) -> Self {
        P2PBehaviourEvent::ReqResp(event)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[derive(Clone)]
pub struct HandshakeRequest {
    pub sender: String,
    pub amount_prime: u64,
    pub unit: Unit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub receiver: String,
    pub accepted: bool,
    pub handshake_id: String,
}

pub struct P2PNode {
    pub swarm: Swarm<P2PBehaviour>,
    pub peer_id: PeerId,
}

impl P2PNode {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(1))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .build()
                    .unwrap();

                let mut gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                ).unwrap();

                let topic = gossipsub::IdentTopic::new("primetime-dag");
                gossipsub.subscribe(&topic).unwrap();

                let store = MemoryStore::new(local_peer_id);
                let kademlia = kad::Behaviour::new(local_peer_id, store);

                let req_resp = request_response::cbor::Behaviour::new(
                    [(libp2p::StreamProtocol::new("/primetime/handshake/1.0.0"), request_response::ProtocolSupport::Full)],
                    request_response::Config::default(),
                );

                P2PBehaviour { gossipsub, kademlia, req_resp }
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        Ok(P2PNode { swarm, peer_id: local_peer_id })
    }
}
