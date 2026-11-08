use anyhow::{Context, anyhow};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};

/// The one password rule, for the super admin and every account.
pub const MIN_PASSWORD_CHARS: usize = 12;

/// The hash of a random, discarded password, verified when a login names an
/// unknown user so that it takes as long as a wrong password for a real one.
/// Its parameters must stay those of [`hash`], which a test checks.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$Tqh6p5XHi3Dl78bgBRoViw$KtfE1KceUUV6zHodS7Q192s0cYaQ8QTLRx7dIcDDhsY";

#[must_use]
pub fn is_long_enough(password: &str) -> bool {
    password.chars().count() >= MIN_PASSWORD_CHARS
}

/// Hashes a password with argon2id, in PHC string format.
///
/// Argon2 is deliberately slow: it runs on the blocking pool so that it does
/// not stall the async workers.
pub async fn hash(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        hasher()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|e| anyhow!("failed to hash a password: {e}"))
    })
    .await
    .context("the password hashing task failed")?
}

/// Whether `password` matches `hash`. A hash that cannot be parsed matches
/// nothing.
pub async fn verify(password: String, hash: String) -> anyhow::Result<bool> {
    tokio::task::spawn_blocking(move || {
        hasher()
            .verify_password(password.as_bytes(), hash.as_str())
            .is_ok()
    })
    .await
    .context("the password verification task failed")
}

/// Spends the time of a failed [`verify`], for a login naming no account.
pub async fn verify_dummy(password: String) -> anyhow::Result<()> {
    verify(password, DUMMY_HASH.to_owned()).await.map(|_| ())
}

fn hasher() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

#[cfg(test)]
mod tests {
    use argon2::{Params, PasswordHash};

    use super::DUMMY_HASH;

    #[test]
    fn the_dummy_hash_costs_as_much_as_a_real_one() {
        let dummy = PasswordHash::new(DUMMY_HASH).unwrap();
        let params = Params::try_from(&dummy).unwrap();
        let default = Params::default();
        assert_eq!(dummy.algorithm.as_str(), "argon2id");
        assert_eq!(params.m_cost(), default.m_cost());
        assert_eq!(params.t_cost(), default.t_cost());
        assert_eq!(params.p_cost(), default.p_cost());
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert!(super::is_long_enough("éééééééééééé"));
        assert!(!super::is_long_enough("ééééééééééé"));
    }
}
