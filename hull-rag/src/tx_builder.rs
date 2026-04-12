//! RAG-specific transaction builder — settlement orchestration.
//!
//! Re-exports generic tx_builder from vesl-mantle and adds:
//! - SettlementTxParams (references hull-rag's SettlementData)
//! - build_settlement_tx (orchestrates kernel pokes + NoteData encoding)
//! - settlement_to_note_data (thin wrapper)
//! - Diagnostic poke helpers

pub use vesl_mantle::tx_builder::{
    kernel_sig_hash, kernel_tx_id, jam_seeds_manual, jam_spends_manual,
    extract_hash_from_effect, bytes_to_atom,
};

use nockapp::noun::slab::{NockJammer, NounSlab};
use nockapp::wire::{SystemWire, Wire};
use nockapp::NockApp;
use nockchain_math::belt::Belt;
use nockchain_types::tx_engine::common::{Hash, Name, Nicks};
use nockchain_types::tx_engine::v1::note::NoteData;
use nockchain_types::tx_engine::v1::tx::{
    Lock, LockMerkleProof, LockMerkleProofFull, MerkleProof, PkhSignature, PkhSignatureEntry,
    Seed, Seeds, Spend, Spend1, Spends, SpendCondition, Witness,
};
use nockchain_types::tx_engine::v1::{RawTx, Version};
use nockvm::ext::make_tas;
use nockvm::noun::{D, T};
use nockvm_macros::tas;
use noun_serde::NounEncode;

use crate::chain::SettlementData;
use crate::signing;

// ---------------------------------------------------------------------------
// Settlement transaction builder
// ---------------------------------------------------------------------------

/// Parameters for building a settlement transaction.
pub struct SettlementTxParams {
    /// The coinbase UTXO to spend (note name).
    pub input_name: Name,
    /// The input note's hash (parent-hash for the output seed).
    pub input_note_hash: Hash,
    /// The input note's amount.
    pub input_amount: u64,
    /// Whether the input is a coinbase note.
    pub is_coinbase: bool,
    /// Coinbase relative timelock minimum (only used if `is_coinbase` is true).
    /// Fakenet default is 1.
    pub coinbase_timelock_min: u64,
    /// The input note's source hash (tx hash or block hash for coinbase).
    pub source_hash: Hash,
    /// The recipient's PKH (public key hash) — typically self-send.
    pub recipient_pkh: Hash,
    /// The Vesl settlement data to embed.
    pub settlement: SettlementData,
    /// Transaction fee.
    pub fee: u64,
    /// The signing secret key (8 x 32-bit Belt chunks).
    pub signing_key: [Belt; 8],
}

/// Build a signed settlement transaction with Vesl NoteData.
///
/// Uses the Hoon kernel for sig-hash and tx-id computation (two pokes):
/// 1. `%sig-hash` — compute signing message from seeds + fee
/// 2. `%tx-id` — compute transaction ID from complete spends
pub async fn build_settlement_tx(
    app: &mut NockApp,
    params: &SettlementTxParams,
) -> anyhow::Result<RawTx> {
    // 1. Derive signing pubkey and PKH
    let pubkey = signing::derive_pubkey(&params.signing_key);
    let pkh = signing::pubkey_hash(&pubkey);

    // 2. Build OUTPUT lock (simple P2PKH for recipient — settlement output)
    let recipient_condition = SpendCondition::simple_pkh(params.recipient_pkh.clone());
    let output_lock_root = Lock::SpendCondition(recipient_condition.clone())
        .hash()
        .map_err(|e| anyhow::anyhow!("output lock hash failed: {e}"))?;

    // 3. Build INPUT lock (must match the UTXO being spent)
    let input_condition = if params.is_coinbase {
        SpendCondition::coinbase_pkh(pkh.clone(), params.coinbase_timelock_min)
    } else {
        SpendCondition::simple_pkh(pkh.clone())
    };
    let input_lock_root = Lock::SpendCondition(input_condition.clone())
        .hash()
        .map_err(|e| anyhow::anyhow!("input lock hash failed: {e}"))?;

    // 4. Encode Vesl settlement as NoteData entries
    let note_data = settlement_to_note_data(&params.settlement);

    // 5. Build the output seed
    if params.fee > params.input_amount / 2 {
        return Err(anyhow::anyhow!(
            "fee ({}) exceeds 50% of input amount ({})",
            params.fee, params.input_amount
        ));
    }
    let output_amount = params.input_amount.saturating_sub(params.fee);
    let seed = Seed {
        output_source: None,
        lock_root: output_lock_root,
        note_data,
        gift: Nicks(output_amount as usize),
        parent_hash: params.input_note_hash.clone(),
    };

    let seeds = Seeds(vec![seed.clone()]);
    let fee = Nicks(params.fee as usize);

    // 6. Compute sig-hash via Hoon kernel
    let sig_hash = kernel_sig_hash(app, &seeds, &fee).await?;

    // 7. Sign the sig-hash
    let msg_belts = sig_hash.to_array().map(Belt);
    let signature = signing::sign(&params.signing_key, &msg_belts)
        .map_err(|e| anyhow::anyhow!("signing failed: {e}"))?;

    // 8. Build witness
    let lock_merkle_proof = LockMerkleProofFull {
        version: tas!(b"full"),
        spend_condition: input_condition,
        axis: 1,
        proof: MerkleProof {
            root: input_lock_root,
            path: vec![],
        },
    };

    let pkh_sig_entry = PkhSignatureEntry {
        hash: pkh,
        pubkey,
        signature,
    };

    let witness = Witness::new(
        LockMerkleProof::Full(lock_merkle_proof),
        PkhSignature::new(vec![pkh_sig_entry]),
        vec![],
    );

    // 9. Build spend
    let spend = Spend::Witness(Spend1 {
        witness,
        seeds,
        fee,
    });

    let spends = Spends(vec![(params.input_name.clone(), spend)]);

    // 10. Compute transaction ID via Hoon kernel
    let tx_id = kernel_tx_id(app, &spends).await?;

    // 11. Assemble transaction
    Ok(RawTx {
        version: Version::V1,
        id: tx_id,
        spends,
    })
}

// ---------------------------------------------------------------------------
// NoteData encoding for Vesl settlement
// ---------------------------------------------------------------------------

/// Encode Vesl SettlementData as NoteData entries (JAM'd nouns).
pub fn settlement_to_note_data(settlement: &SettlementData) -> NoteData {
    settlement.to_note_data()
}

// ---------------------------------------------------------------------------
// Diagnostic pokes — isolate crash in %sig-hash pipeline
// ---------------------------------------------------------------------------

/// Expose seeds JAM for diagnostic use (returns raw bytes).
pub fn jam_seeds_for_diag(seeds: &Seeds) -> anyhow::Result<bytes::Bytes> {
    jam_seeds_manual(seeds)
}

/// Fire `%diag-cue` to CUE the seeds JAM without applying the type sieve.
pub async fn kernel_diag_cue(
    app: &mut NockApp,
    seeds: &Seeds,
) -> anyhow::Result<Vec<NounSlab>> {
    let seeds_jammed = jam_seeds_manual(seeds)?;
    let mut poke_slab: NounSlab = NounSlab::new();
    let tag = make_tas(&mut poke_slab, "diag-cue").as_noun();
    let seeds_atom = bytes_to_atom(&mut poke_slab, &seeds_jammed);
    let cmd = T(&mut poke_slab, &[tag, seeds_atom]);
    poke_slab.set_root(cmd);
    app.poke(SystemWire.to_wire(), poke_slab)
        .await
        .map_err(|e| anyhow::anyhow!("diag-cue poke failed: {e:?}"))
}

/// Fire `%diag-hash` to test sig-hashable + hash-hashable inside mules.
pub async fn kernel_diag_hash(
    app: &mut NockApp,
    seeds: &Seeds,
    fee: &nockchain_types::tx_engine::common::Nicks,
) -> anyhow::Result<Vec<NounSlab>> {
    let seeds_jammed = jam_seeds_manual(seeds)?;
    let mut poke_slab: NounSlab = NounSlab::new();
    let tag = make_tas(&mut poke_slab, "diag-hash").as_noun();
    let seeds_atom = bytes_to_atom(&mut poke_slab, &seeds_jammed);
    let fee_noun = D(fee.0 as u64);
    let cmd = T(&mut poke_slab, &[tag, seeds_atom, fee_noun]);
    poke_slab.set_root(cmd);
    app.poke(SystemWire.to_wire(), poke_slab)
        .await
        .map_err(|e| anyhow::anyhow!("diag-hash poke failed: {e:?}"))
}

/// Fire `%diag-sieve` to CUE and apply `;;(seeds:txv1 ...)` inside a mule.
pub async fn kernel_diag_sieve(
    app: &mut NockApp,
    seeds: &Seeds,
) -> anyhow::Result<Vec<NounSlab>> {
    let seeds_jammed = jam_seeds_manual(seeds)?;
    let mut poke_slab: NounSlab = NounSlab::new();
    let tag = make_tas(&mut poke_slab, "diag-sieve").as_noun();
    let seeds_atom = bytes_to_atom(&mut poke_slab, &seeds_jammed);
    let cmd = T(&mut poke_slab, &[tag, seeds_atom]);
    poke_slab.set_root(cmd);
    app.poke(SystemWire.to_wire(), poke_slab)
        .await
        .map_err(|e| anyhow::anyhow!("diag-sieve poke failed: {e:?}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use nockchain_types::tx_engine::common::Hash;

    #[test]
    fn settlement_note_data_encodes() {
        let settlement = SettlementData {
            version: 1,
            hull_id: 7,
            merkle_root: [1, 2, 3, 4, 5],
            note_id: 42,
            manifest_hash: [6, 7, 8, 9, 10],
            proof_jam: None,
        };
        let nd = settlement_to_note_data(&settlement);
        assert_eq!(nd.0.len(), 5);
    }

    /// Verify `jam_seeds_manual` succeeds with 6 NoteData entries (proof present).
    #[test]
    fn jam_seeds_manual_succeeds_with_proof() {
        let mock_proof = bytes::Bytes::from(
            [0xDE, 0xAD, 0xBE, 0xEF, 0x42]
                .iter()
                .copied()
                .cycle()
                .take(500)
                .collect::<Vec<u8>>(),
        );
        let settlement = SettlementData {
            version: 2,
            hull_id: 7,
            merkle_root: [100, 200, 300, 400, 500],
            note_id: 42,
            manifest_hash: [10, 20, 30, 40, 50],
            proof_jam: Some(mock_proof),
        };
        let note_data = settlement_to_note_data(&settlement);
        assert_eq!(
            note_data.0.len(),
            6,
            "should have 6 entries (5 metadata + proof)"
        );

        let seed = Seed {
            output_source: None,
            lock_root: Hash::from_limbs(&[1, 2, 3, 4, 5]),
            note_data,
            gift: Nicks(62_536),
            parent_hash: Hash::from_limbs(&[10, 20, 30, 40, 50]),
        };
        let seeds = Seeds(vec![seed]);

        let manual_jam = jam_seeds_manual(&seeds).expect("manual JAM should succeed with proof");
        assert!(
            manual_jam.len() > 100,
            "JAM'd seeds with proof should be non-trivial (got {} bytes)",
            manual_jam.len()
        );
    }

    /// Belt-list encoding round-trips: encode proof -> note-data -> decode matches original.
    #[test]
    fn proof_belt_encoding_round_trips() {
        let patterns: Vec<Vec<u8>> = vec![
            vec![0x01, 0x02, 0x03],
            vec![0xDE, 0xAD, 0xBE, 0xEF, 0x42, 0x13, 0x37],
            [0xDE, 0xAD, 0xBE, 0xEF, 0x42]
                .iter()
                .copied()
                .cycle()
                .take(500)
                .collect(),
            vec![],
        ];
        for original in &patterns {
            let settlement = SettlementData {
                version: 2,
                hull_id: 7,
                merkle_root: [100, 200, 300, 400, 500],
                note_id: 42,
                manifest_hash: [10, 20, 30, 40, 50],
                proof_jam: if original.is_empty() {
                    None
                } else {
                    Some(bytes::Bytes::from(original.clone()))
                },
            };
            let note_data = settlement.to_note_data();
            let decoded = SettlementData::from_note_data(&note_data)
                .expect("round-trip decode should succeed");
            match (&settlement.proof_jam, &decoded.proof_jam) {
                (None, None) => {}
                (Some(orig), Some(got)) => {
                    assert_eq!(
                        orig.as_ref(),
                        got.as_ref(),
                        "proof bytes mismatch for input of length {}",
                        original.len()
                    );
                }
                _ => panic!("proof_jam presence mismatch"),
            }
        }
    }

    /// A1b diagnostic: Same comparison for `jam_spends_manual` vs `Spends::to_noun`.
    #[test]
    fn jam_spends_manual_matches_spends_to_noun() {
        let settlement = SettlementData {
            version: 1,
            hull_id: 7,
            merkle_root: [100, 200, 300, 400, 500],
            note_id: 42,
            manifest_hash: [10, 20, 30, 40, 50],
            proof_jam: None,
        };
        let note_data = settlement_to_note_data(&settlement);

        let seed = Seed {
            output_source: None,
            lock_root: Hash::from_limbs(&[1, 2, 3, 4, 5]),
            note_data,
            gift: Nicks(62_536),
            parent_hash: Hash::from_limbs(&[10, 20, 30, 40, 50]),
        };
        let seeds = Seeds(vec![seed]);
        let fee = Nicks(3000);

        let pkh = Hash::from_limbs(&[99, 88, 77, 66, 55]);
        let input_condition = SpendCondition::coinbase_pkh(pkh.clone(), 1);
        let input_lock_root = Lock::SpendCondition(input_condition.clone())
            .hash()
            .expect("lock hash");

        let lock_merkle_proof = LockMerkleProofFull {
            version: nockvm_macros::tas!(b"full"),
            spend_condition: input_condition,
            axis: 1,
            proof: MerkleProof {
                root: input_lock_root,
                path: vec![],
            },
        };

        let dummy_sig = nockchain_types::tx_engine::common::SchnorrSignature {
            chal: [nockchain_math::belt::Belt(1); 8],
            sig: [nockchain_math::belt::Belt(2); 8],
        };
        let dummy_pk = crate::signing::derive_pubkey(&crate::signing::demo_signing_key());

        let pkh_sig_entry = PkhSignatureEntry {
            hash: pkh,
            pubkey: dummy_pk,
            signature: dummy_sig,
        };

        let witness = Witness::new(
            LockMerkleProof::Full(lock_merkle_proof),
            PkhSignature::new(vec![pkh_sig_entry]),
            vec![],
        );

        let spend = Spend::Witness(Spend1 {
            witness,
            seeds,
            fee,
        });

        let name = nockchain_types::tx_engine::common::Name::new(
            Hash::from_limbs(&[1, 1, 1, 1, 1]),
            Hash::from_limbs(&[2, 2, 2, 2, 2]),
        );
        let spends = Spends(vec![(name, spend)]);

        // Path 1: manual JAM
        let manual_jam = jam_spends_manual(&spends).expect("manual JAM should succeed");

        // Path 2: Spends::to_noun -> JAM
        let standard_jam = {
            let mut slab: NounSlab<NockJammer> = NounSlab::new();
            let noun = spends.to_noun(&mut slab);
            slab.set_root(noun);
            slab.jam()
        };

        assert_eq!(
            manual_jam.to_vec(),
            standard_jam.to_vec(),
            "jam_spends_manual must produce identical bytes to Spends::to_noun -> JAM."
        );
    }
}
