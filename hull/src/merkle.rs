//! Merkle tree engine — tip5-based Merkle tree for cross-runtime alignment.

pub use nockchain_tip5_rs::{
    hash_leaf, hash_pair, verify_proof, MerkleTree, format_tip5,
    Tip5Hash, TIP5_ZERO, ProofNode,
    tip5_to_atom_le_bytes,
};
