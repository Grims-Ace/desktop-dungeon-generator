/*
 * Deterministic seed management.
 * Provides functions to generate random master seeds and
 * derive sub-seeds for specific procedural generation steps.
 */

use std::time::{SystemTime, UNIX_EPOCH};

// Generate a random master seed from time and process id.
pub fn random_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    mix64(nanos ^ pid.rotate_left(17))
}

// Derive a deterministic seed for a specific random stream.
pub fn derive_seed(master_seed: u64, stream_id: u64) -> u64 {
    mix64(master_seed ^ stream_id.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

// Mix bits for a 64-bit seed hash.
fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}
