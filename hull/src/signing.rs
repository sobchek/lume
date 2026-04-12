//! Signing — re-exports generic signing from vesl-core + demo key.

pub use vesl_core::signing::{
    SigningError, derive_pubkey, pubkey_hash, sign, key_from_seed_phrase,
};

use nockchain_math::belt::Belt;

// ---------------------------------------------------------------------------
// Demo signing key — deterministic key for fakenet testing
// ---------------------------------------------------------------------------

/// The demo signing key used for fakenet settlement transactions.
///
/// This is a deterministic key (sk[0]=12345, sk[1]=67890) whose PKH can be
/// used as the `--mining-pkh` when starting the fakenet miner. This ensures
/// the hull can spend mined coinbase UTXOs.
pub fn demo_signing_key() -> [Belt; 8] {
    let mut sk = [Belt(0); 8];
    sk[0] = Belt(12345);
    sk[1] = Belt(67890);
    sk
}

/// Base58-encoded PKH of the demo signing key.
///
/// Use this as `--mining-pkh` when starting the fakenet miner.
pub const DEMO_KEY_PKH_BASE58: &str = "5pJiNWqnouxku6SvGU6XZhu98nHH5VFMaNJ4r1vtHxPJ5sHurHBfYnk";

/// Check whether a signing key matches the hardcoded demo key.
pub fn is_demo_key(sk: &[Belt; 8]) -> bool {
    *sk == demo_signing_key()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_key_pkh_base58() {
        let mut sk = [Belt(0); 8];
        sk[0] = Belt(12345);
        sk[1] = Belt(67890);
        let pk = derive_pubkey(&sk);
        let pkh = pubkey_hash(&pk);
        let pkh_b58 = pkh.to_base58();
        println!("DEMO_KEY_PKH_BASE58={pkh_b58}");
        assert!(!pkh_b58.is_empty());
    }
}
