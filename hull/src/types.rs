//! Rust mirrors of the Hoon data structures from protocol/sur/vesl.hoon.

use serde::{Deserialize, Serialize};

pub use nockchain_tip5_rs::{ProofNode, Tip5Hash, TIP5_ZERO};

/// Mirror of `+$chunk  [id=chunk-id dat=@t]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: u64,
    pub dat: String,
}

/// Mirror of `+$retrieval  [=chunk proof=merkle-proof score=@ud]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retrieval {
    pub chunk: Chunk,
    pub proof: Vec<ProofNode>,
    pub score: u64,
}

/// Mirror of `+$manifest  [query=@t results=(list retrieval) prompt=@t output=@t page=@ud]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub query: String,
    pub results: Vec<Retrieval>,
    pub prompt: String,
    pub output: String,
    pub page: u64,
}

/// Mirror of `+$nock-zkp  [root=merkle-root prf=@ stamp=@da]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NockZkp {
    pub root: Tip5Hash,
    pub prf: Vec<u8>,
    pub stamp: u64,
}

/// Mirror of `+$note-state`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteState {
    Pending,
    Verified(NockZkp),
    Settled,
}

/// Mirror of `+$note  [id=@ hull=hull-id root=merkle-root state=note-state]`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub hull: u64,
    pub root: Tip5Hash,
    pub state: NoteState,
}
