use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};

/// Implements a simple Proof of Sequential Work (PoSW).
/// Hashes the seed N times: H(H(H(...)))
pub fn generate_effort(seed: &[u8], iterations: u64) -> [u8; 32] {
    let mut current_hash = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(seed);
    current_hash.copy_from_slice(&hasher.finalize());

    for _ in 1..iterations {
        let mut hasher = Sha256::new();
        hasher.update(&current_hash);
        current_hash.copy_from_slice(&hasher.finalize());
    }

    current_hash
}

/// A node's identity containing its keypair.
pub struct NodeIdentity {
    pub keypair: SigningKey,
}

impl NodeIdentity {
    pub fn generate() -> Self {
        let mut csprng = OsRng {};
        let keypair = SigningKey::generate(&mut csprng);
        Self { keypair }
    }

    pub fn public_key(&self) -> VerifyingKey {
        self.keypair.verifying_key()
    }
}

/// Proof of Sequential Work struct
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffortProof {
    pub seed: Vec<u8>,
    pub iterations: u64,
    pub final_hash: [u8; 32],
}

impl EffortProof {
    pub fn verify(&self) -> bool {
        let expected_hash = generate_effort(&self.seed, self.iterations);
        self.final_hash == expected_hash
    }
}

/// Represents a "Mint" operation when a node crosses the prime scarcity threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultBookMint {
    pub public_key: [u8; 32], // ed25519 public key bytes
    pub proof: EffortProof,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],  // ed25519 signature
}

impl VaultBookMint {
    pub fn new(identity: &NodeIdentity, proof: EffortProof) -> Self {
        let mut data = proof.seed.clone();
        data.extend_from_slice(&proof.iterations.to_le_bytes());
        data.extend_from_slice(&proof.final_hash);

        let signature = identity.keypair.sign(&data).to_bytes();
        Self {
            public_key: identity.public_key().to_bytes(),
            proof,
            signature,
        }
    }

    pub fn verify(&self) -> bool {
        if !self.proof.verify() {
            return false;
        }

        if let Ok(public_key) = VerifyingKey::from_bytes(&self.public_key) {
            let mut data = self.proof.seed.clone();
            data.extend_from_slice(&self.proof.iterations.to_le_bytes());
            data.extend_from_slice(&self.proof.final_hash);

            let signature = Signature::from_bytes(&self.signature);
            return public_key.verify_strict(&data, &signature).is_ok();
        }

        false
    }
}

/// Represents a transfer of "counts" between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicTransfer {
    pub source_pubkey: [u8; 32],
    pub destination_pubkey: [u8; 32],
    pub amount: u64,
    pub nonce: u64,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}

impl HeuristicTransfer {
    pub fn new(
        identity: &NodeIdentity,
        destination_pubkey: [u8; 32],
        amount: u64,
        nonce: u64,
    ) -> Self {
        let mut data = destination_pubkey.to_vec();
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&nonce.to_le_bytes());

        let signature = identity.keypair.sign(&data).to_bytes();
        Self {
            source_pubkey: identity.public_key().to_bytes(),
            destination_pubkey,
            amount,
            nonce,
            signature,
        }
    }

    pub fn verify(&self) -> bool {
        if let Ok(public_key) = VerifyingKey::from_bytes(&self.source_pubkey) {
            let mut data = self.destination_pubkey.to_vec();
            data.extend_from_slice(&self.amount.to_le_bytes());
            data.extend_from_slice(&self.nonce.to_le_bytes());

            let signature = Signature::from_bytes(&self.signature);
            return public_key.verify_strict(&data, &signature).is_ok();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_vdf_measurable_time_and_verification() {
        let seed = b"alpha_book_seed";
        let iterations = 10_000;

        let start = Instant::now();
        let final_hash = generate_effort(seed, iterations);
        let duration = start.elapsed();

        assert!(duration.as_micros() > 0, "VDF should take measurable time");

        let proof = EffortProof {
            seed: seed.to_vec(),
            iterations,
            final_hash,
        };

        assert!(proof.verify(), "VDF proof should verify deterministically");
    }

    #[test]
    fn test_vault_book_mint() {
        let identity = NodeIdentity::generate();
        let proof = EffortProof {
            seed: b"test_seed".to_vec(),
            iterations: 100,
            final_hash: generate_effort(b"test_seed", 100),
        };

        let mint = VaultBookMint::new(&identity, proof);
        assert!(mint.verify(), "Valid mint should pass verification");
    }

    #[test]
    fn test_heuristic_transfer() {
        let identity1 = NodeIdentity::generate();
        let identity2 = NodeIdentity::generate();

        let transfer = HeuristicTransfer::new(&identity1, identity2.public_key().to_bytes(), 500, 1);
        assert!(transfer.verify(), "Valid transfer should pass verification");
    }
}
