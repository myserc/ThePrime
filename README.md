# ThePrime
An Intellectual Property Organisation

## Operation Autarky-One: primetime-node

A decentralized, peer-to-peer (P2P) network binary capable of verifiable work and distributed thermodynamic consensus.

### Installation & Usage

1. **Prerequisites**
   Ensure you have Rust and Cargo installed. If not, install via rustup:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs
   ```

2. **Build**
   Compile the node application in release mode for maximum hashing performance:
   ```bash
   cd primetime-node
   cargo build --release
   ```

3. **Running the Node**
   Start a local node instance. Provide a port for the UI Server and a port for the libp2p Swarm:
   ```bash
   # Usage: ./primetime-node <UI_PORT> <SWARM_PORT> [BOOTNODE_MULTIADDR]
   cargo run --release -- 3001 0
   ```

4. **Multi-Node Deployment**
   If you wish to test a local multi-node cluster, you can supply the Multiaddr of an active node to bootstrap the connection over Kademlia DHT.

   *Node 1 (Bootnode):*
   ```bash
   cargo run --release -- 3001 4001
   # Note the Node ID printed, e.g., 12D3KooW...
   ```

   *Node 2 (Connecting Node):*
   ```bash
   cargo run --release -- 3002 4002 /ip4/127.0.0.1/tcp/4001/p2p/12D3KooW...
   ```

5. **Telemetry Dashboard**
   Once a node is running, open a web browser to its designated UI port (e.g., `http://localhost:3001`) to view the real-time block-DAG validation, hash rate, local vault balance, and system-wide entropy metrics.
