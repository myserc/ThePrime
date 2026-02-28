use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof {
    pub start_hash: String,
    pub end_hash: String,
    pub iterations: usize,
}

pub fn mine_iterations(start_hash: &str, target_iterations: usize) -> Proof {
    let mut current_hash = start_hash.to_string();

    for _ in 0..target_iterations {
        let mut hasher = Sha256::new();
        hasher.update(current_hash.as_bytes());
        let result = hasher.finalize();
        current_hash = hex::encode(result);
    }

    Proof {
        start_hash: start_hash.to_string(),
        end_hash: current_hash,
        iterations: target_iterations,
    }
}

pub fn verify_proof(proof: &Proof) -> bool {
    let mut current_hash = proof.start_hash.clone();

    for _ in 0..proof.iterations {
        let mut hasher = Sha256::new();
        hasher.update(current_hash.as_bytes());
        let result = hasher.finalize();
        current_hash = hex::encode(result);
    }

    current_hash == proof.end_hash
}
