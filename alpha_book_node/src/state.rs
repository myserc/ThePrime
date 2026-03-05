use crate::crypto::{HeuristicTransfer, VaultBookMint};
use sled::Db;
use std::sync::Arc;
use tracing::{info, warn};

const PRIME_SCARCITY_THRESHOLD: u64 = 9_731_081;

#[derive(Clone)]
pub struct LocalState {
    db: Arc<Db>,
}

impl LocalState {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db: Arc::new(db) })
    }

    /// Helper to get the balance of a given public key
    pub fn get_balance(&self, pubkey: &[u8; 32]) -> anyhow::Result<u64> {
        if let Some(val) = self.db.get(pubkey)? {
            let balance: u64 = bincode::deserialize(&val)?;
            Ok(balance)
        } else {
            Ok(0) // Default balance
        }
    }

    /// Helper to set the balance of a given public key
    pub fn set_balance(&self, pubkey: &[u8; 32], balance: u64) -> anyhow::Result<()> {
        let val = bincode::serialize(&balance)?;
        self.db.insert(pubkey, val)?;
        Ok(())
    }

    /// Get the global vault book count
    pub fn get_global_vault_books(&self) -> anyhow::Result<u64> {
        if let Some(val) = self.db.get(b"global_vault_books")? {
            let count: u64 = bincode::deserialize(&val)?;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Increment the global vault book count
    pub fn increment_global_vault_books(&self) -> anyhow::Result<()> {
        let current = self.get_global_vault_books()?;
        let val = bincode::serialize(&(current + 1))?;
        self.db.insert(b"global_vault_books", val)?;
        Ok(())
    }

    /// Process a VaultBookMint
    pub fn process_mint(&self, mint: &VaultBookMint) -> anyhow::Result<bool> {
        if !mint.verify() {
            warn!("Invalid VaultBookMint signature or PoSW proof");
            return Ok(false);
        }

        // The requirement is that the proof itself "crosses the prime threshold"
        // Here we just make sure the iterations are at least the threshold for simplicity,
        // or you could have a dynamic difficulty. For this spec:
        if mint.proof.iterations < PRIME_SCARCITY_THRESHOLD {
            warn!("VaultBookMint iterations do not meet the PRIME_SCARCITY_THRESHOLD");
            return Ok(false);
        }

        // To prevent replay attacks, we should ensure this exact mint hasn't been processed.
        // We can use the mint's signature as a unique ID.
        let mint_id = &mint.signature;
        if self.db.contains_key(mint_id)? {
            warn!("VaultBookMint already processed");
            return Ok(false);
        }

        // Mark mint as processed
        self.db.insert(mint_id, vec![1])?;

        // Increment global vault books
        self.increment_global_vault_books()?;

        // Reset minter's local momentum (balance)
        self.set_balance(&mint.public_key, 0)?;

        info!("Processed VaultBookMint successfully. Global vault books incremented.");
        Ok(true)
    }

    /// Process a HeuristicTransfer
    pub fn process_transfer(&self, transfer: &HeuristicTransfer) -> anyhow::Result<bool> {
        if !transfer.verify() {
            warn!("Invalid HeuristicTransfer signature");
            return Ok(false);
        }

        // Replay protection
        let transfer_id = &transfer.signature;
        if self.db.contains_key(transfer_id)? {
            warn!("HeuristicTransfer already processed");
            return Ok(false);
        }

        let source_balance = self.get_balance(&transfer.source_pubkey)?;
        if source_balance < transfer.amount {
            warn!("Insufficient balance for transfer");
            return Ok(false);
        }

        // Deduct from source
        self.set_balance(&transfer.source_pubkey, source_balance - transfer.amount)?;

        // Add to destination
        let dest_balance = self.get_balance(&transfer.destination_pubkey)?;
        self.set_balance(&transfer.destination_pubkey, dest_balance + transfer.amount)?;

        // Mark transfer as processed
        self.db.insert(transfer_id, vec![1])?;

        info!("Processed HeuristicTransfer successfully.");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{EffortProof, NodeIdentity, generate_effort};
    use tempfile::tempdir;

    #[test]
    fn test_state_transitions() {
        let dir = tempdir().unwrap();
        let state = LocalState::new(dir.path().to_str().unwrap()).unwrap();

        let node1 = NodeIdentity::generate();
        let node2 = NodeIdentity::generate();

        // 1. Initial balance check
        assert_eq!(state.get_balance(&node1.public_key().to_bytes()).unwrap(), 0);
        assert_eq!(state.get_global_vault_books().unwrap(), 0);

        // 2. Set some initial balance for node1
        state.set_balance(&node1.public_key().to_bytes(), 1000).unwrap();
        assert_eq!(state.get_balance(&node1.public_key().to_bytes()).unwrap(), 1000);

        // 3. Test HeuristicTransfer
        let transfer = HeuristicTransfer::new(&node1, node2.public_key().to_bytes(), 300, 1);
        let success = state.process_transfer(&transfer).unwrap();
        assert!(success);

        assert_eq!(state.get_balance(&node1.public_key().to_bytes()).unwrap(), 700);
        assert_eq!(state.get_balance(&node2.public_key().to_bytes()).unwrap(), 300);

        // Try double spend / replay
        let success_replay = state.process_transfer(&transfer).unwrap();
        assert!(!success_replay);

        // 4. Test VaultBookMint
        let seed = b"test_seed";
        let iterations = PRIME_SCARCITY_THRESHOLD;
        let hash = generate_effort(seed, iterations);
        let proof = EffortProof {
            seed: seed.to_vec(),
            iterations,
            final_hash: hash,
        };
        let mint = VaultBookMint::new(&node1, proof);

        let success_mint = state.process_mint(&mint).unwrap();
        assert!(success_mint);

        // Balance should be reset to 0
        assert_eq!(state.get_balance(&node1.public_key().to_bytes()).unwrap(), 0);
        // Global vault books should be 1
        assert_eq!(state.get_global_vault_books().unwrap(), 1);

        // Replay mint
        let success_mint_replay = state.process_mint(&mint).unwrap();
        assert!(!success_mint_replay);
    }
}
