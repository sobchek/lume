//! Kraken — STARK Proof helpers (heaviest tentacle)
//!
//! Provides composable helpers for STARK proof handling:
//! - Extract proof bytes from kernel effects after a %prove poke
//! - Build a generic %prove poke from pre-jammed payload bytes
//!
//! The hull fires the %prove poke and manages the NockApp. Kraken
//! provides the effect parsing and poke construction. Kernel boot
//! (including prover jet registration) is the hull's responsibility.

use anyhow::Result;
use nock_noun_rs::{make_atom_in, NounSlab, T};

/// Extract proof bytes from kernel effects after a %prove poke.
///
/// Expected effect shape on success: `[result-note proof-atom]`
/// Expected effect shape on failure: `[%prove-failed ~]`
///
/// Returns `Ok(Some(bytes))` if proof was extracted,
/// `Ok(None)` if the effect indicates failure (prove-failed),
/// `Err` if the effect structure is unexpected.
pub fn extract_proof_from_effects(effects: &[NounSlab]) -> Result<Option<bytes::Bytes>> {
    let effect_slab = match effects.first() {
        Some(slab) => slab,
        None => return Ok(None),
    };

    // SAFETY: effect_slab comes from NockApp::poke, which sets the root.
    let root_noun = unsafe { *effect_slab.root() };

    let cell = match root_noun.as_cell() {
        Ok(c) => c,
        Err(_) => return Ok(None), // atom effect = no proof
    };

    // Check for failure tag: [%prove-failed ...]
    if let Ok(tag_atom) = cell.head().as_atom() {
        if let Ok(tag_val) = tag_atom.as_u64() {
            // %prove-failed = 0x6465_6c69_6166_2d65_766f_7270 (too large for u64)
            // Check by trying to read as bytes
            let tag_bytes = tag_atom.as_ne_bytes();
            if tag_bytes.starts_with(b"prove-failed") {
                return Ok(None);
            }
            // If head is a small atom, this isn't [note proof] — might be a tag
            if tag_val > 0 && cell.head().is_atom() {
                // Could be a cell where head is an atom (tag). Check tail.
                let tail = cell.tail();
                if let Ok(proof_atom) = tail.as_atom() {
                    let bytes = proof_atom.as_ne_bytes();
                    let len = bytes
                        .iter()
                        .rposition(|&b| b != 0)
                        .map_or(0, |pos| pos + 1);
                    if len > 0 {
                        return Ok(Some(bytes::Bytes::copy_from_slice(&bytes[..len])));
                    }
                }
            }
        }
    }

    // Success: [result-note proof-atom] where result-note is a cell
    if cell.head().is_cell() {
        let proof_noun = cell.tail();
        if let Ok(proof_atom) = proof_noun.as_atom() {
            let bytes = proof_atom.as_ne_bytes();
            let len = bytes
                .iter()
                .rposition(|&b| b != 0)
                .map_or(0, |pos| pos + 1);
            if len > 0 {
                return Ok(Some(bytes::Bytes::copy_from_slice(&bytes[..len])));
            }
        }
    }

    Ok(None)
}

/// Build a generic `%prove` poke from pre-jammed payload bytes.
///
/// Constructs `[%prove jammed-payload-atom]`. The caller jams their
/// domain-specific payload before calling this.
pub fn build_prove_poke_generic(jammed_payload: &[u8]) -> NounSlab {
    let mut slab = NounSlab::new();
    let tag = make_atom_in(&mut slab, b"prove");
    let payload = make_atom_in(&mut slab, jammed_payload);
    let poke = T(&mut slab, &[tag, payload]);
    slab.set_root(poke);
    slab
}

/// Placeholder struct for future full STARK prover integration.
///
/// When the STARK prover is wired in, this struct will hold the
/// hot state (zkvm-jetpack prover context). For now, the hull
/// handles kernel boot with prover jets directly.
pub struct Kraken;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use nock_noun_rs::D;

    #[test]
    fn build_prove_poke_generic_produces_cell() {
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let slab = build_prove_poke_generic(&payload);
        let root = unsafe { slab.root() };
        assert!(root.is_cell(), "prove poke must be a cell [%prove payload]");
    }

    #[test]
    fn extract_proof_from_empty_effects() {
        let effects: Vec<NounSlab> = vec![];
        let result = extract_proof_from_effects(&effects).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn extract_proof_from_cell_with_proof() {
        // Simulate: [result-note proof-atom]
        // result-note is a cell [42 7], proof is an atom with some bytes
        let mut slab = NounSlab::new();
        let note = T(&mut slab, &[D(42), D(7)]);
        let proof_data = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let proof = make_atom_in(&mut slab, &proof_data);
        let effect = T(&mut slab, &[note, proof]);
        slab.set_root(effect);

        let result = extract_proof_from_effects(&[slab]).unwrap();
        assert!(result.is_some());
        let bytes = result.unwrap();
        assert_eq!(&bytes[..], &[0xCA, 0xFE, 0xBA, 0xBE]);
    }
}
