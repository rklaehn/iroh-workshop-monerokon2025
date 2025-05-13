use std::{env, str::FromStr};

use anyhow::{Context, Result};
use iroh::PublicKey;
use iroh_base::SecretKey;
use rand::thread_rng;

/// Gets a secret key from the IROH_SECRET environment variable or generates a new random one.
/// If the environment variable is set, it must be a valid string representation of a secret key.
pub fn get_or_generate_secret_key() -> Result<SecretKey> {
    if let Ok(secret) = env::var("IROH_SECRET") {
        // Parse the secret key from string
        SecretKey::from_str(&secret).context("Invalid secret key format")
    } else {
        // Generate a new random key
        let secret_key = SecretKey::generate(&mut thread_rng());
        println!("Generated new secret key: {}", secret_key);
        println!("To reuse this key, set the IROH_SECRET environment variable to this value");
        Ok(secret_key)
    }
}

/// Print public key (aka node id) as a z32 string, compatible with https://pkarr.org/
pub fn z32_node_id(node_id: &PublicKey) -> String {
    zbase32::encode_full_bytes(node_id.as_bytes().as_slice())
}
