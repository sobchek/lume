//! Vesl NockApp kernel JAM embedding crate.
//!
//! Exposes the compiled kernel bytes plus a compile-time sha256 that
//! `verify_kernel()` checks at runtime. See AUDIT 2026-04-17 M-07.

use sha2::{Digest, Sha256};

/// The compiled vesl kernel JAM, embedded at build time.
pub static KERNEL: &[u8] = include_bytes!(env!("KERNEL_JAM_PATH"));

/// Hex-encoded sha256 of `KERNEL` computed at build time from the JAM
/// file that was embedded.
///
/// Compared by `verify_kernel()` against a runtime-computed digest of
/// `KERNEL`. Divergence means the build-time-embedded bytes differ
/// from what this constant promised — almost certainly JAM tampering
/// between `make kernel` and binary link. Panic is the right response.
pub const KERNEL_SHA256_HEX: &str = env!("KERNEL_JAM_SHA256");

/// Hash `KERNEL` at runtime and check it against `KERNEL_SHA256_HEX`.
///
/// Call once at boot before using the kernel. Panics on mismatch with
/// both hashes in the message for debugging. Cheap — ~20ms for an
/// 18 MB JAM on typical hardware, run-once at startup.
pub fn verify_kernel() {
    let digest = Sha256::digest(KERNEL);
    let actual: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        actual, KERNEL_SHA256_HEX,
        "kernels_vesl: embedded JAM sha256 does not match build-time expected \
         (actual: {actual}, expected: {KERNEL_SHA256_HEX}) — JAM was tampered \
         between build-time hash and binary link, refusing to boot",
    );
}
