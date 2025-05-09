use std::{env, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use iroh_base::SecretKey;
use rand::{thread_rng, Rng};

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

/// Create a unique directory for sending files.
pub fn create_send_dir() -> Result<PathBuf> {
    let suffix = rand::thread_rng().gen::<[u8; 16]>();
    let cwd = std::env::current_dir()?;
    let blobs_data_dir = cwd.join(format!(".{}-send-{}", crate_name(), hex::encode(suffix)));
    if blobs_data_dir.exists() {
        println!(
            "can not share twice from the same directory: {}",
            cwd.display(),
        );
        std::process::exit(1);
    }
    Ok(blobs_data_dir)
}

pub fn crate_name() -> &'static str {
    env!("CARGO_CRATE_NAME")
}
