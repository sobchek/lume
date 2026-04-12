//! Hull — Generic Vesl base template. Fork this.
//!
//! Minimal NockApp hull for verified field commitment on Nockchain.
//! No retriever, no LLM, no RAG. Just commit key-value fields to a
//! Merkle tree, register the root with the kernel, verify, and settle.
//!
//! Community developers: replace `FieldVerifier` with domain logic,
//! swap out the `/commit` endpoint for your data model, and ship.

pub mod api;
pub mod config;
pub mod signing;
pub mod verify;
