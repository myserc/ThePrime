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

### Operations Guide (Minting, Transfers, & Balances)

The PrimeTime decentralized node operates autonomously based on cryptographic Proof of Work (PoW) loops.

- **Minting (Proof-of-Work):**
  The node runs a background `SHA-256` iterative loop. Each successful iteration advances the local "Counts" and updates the node's `prime_value`. Once this value exceeds the defined `STANDARD_MINT_SCARCITY`, the node automatically generates a verifiable cryptographic proof, commits a **Mint Event** to the local DAG ledger, and broadcasts it to the network. As long as the node is running (and has CPU cycles available), it will attempt to mint automatically.

- **Transferring Heuristics:**
  Once connected to peers (via the DHT), the node randomly initiates Request-Response handshakes with connected peers. If a node has a high enough `prime_value`, it will propose a `HandshakeRequest` to trade a "Heuristic" (like `Unit::Day` or `Unit::Degree`). When a peer accepts, the sender automatically deducts their prime value, generates a **Transfer Event**, and broadcasts this transaction over the `primetime-dag` Gossipsub topic. This all happens automatically in the background.

- **Checking Balances (Node Explorer):**
  To view your node's operations and balances, open the Telemetry Dashboard at `http://localhost:<UI_PORT>`.
  - **Local Vault Books:** This is your core balance, incremented automatically upon successful mints.
  - **Prime Value:** The current accumulated thermodynamic momentum. When this reaches the threshold, a new Vault Book is minted.
  - **Hash Rate:** The iterations per second your CPU is performing.
  - **Global Net Entropy:** The real-time aggregate entropy level of your local P2P cluster, synchronized continuously via Gossipsub.
