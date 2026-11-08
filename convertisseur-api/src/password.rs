use anyhow::{Context, anyhow};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};

/// Hashes a password with argon2id, in PHC string format.
///
/// Argon2 is deliberately slow: it runs on the blocking pool so that it does
/// not stall the async workers.
pub async fn hash(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|e| anyhow!("failed to hash a password: {e}"))
    })
    .await
    .context("the password hashing task failed")?
}
