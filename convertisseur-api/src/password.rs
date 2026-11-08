use anyhow::{Context, anyhow};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};

/// The one password rule, for the super admin and every account.
pub const MIN_PASSWORD_CHARS: usize = 12;

/// The hash of a random, discarded password, verified when a login names an
/// unknown user so that it takes as long as a wrong password for a real one.
/// Its parameters must stay those of [`hash`], which a test checks.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$Tqh6p5XHi3Dl78bgBRoViw$KtfE1KceUUV6zHodS7Q192s0cYaQ8QTLRx7dIcDDhsY";

/// Lowercase letters and digits a reader cannot mistake for one another: no
/// `0`/`o`, `1`/`l`/`i`, so that a password read out or copied by hand
/// arrives intact.
const TEMPORARY_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
const TEMPORARY_GROUPS: usize = 4;
const TEMPORARY_GROUP_CHARS: usize = 5;

#[must_use]
pub fn is_long_enough(password: &str) -> bool {
    password.chars().count() >= MIN_PASSWORD_CHARS
}

/// A random password for an admin to hand over: four groups of five
/// characters joined by `-`, about 99 bits of entropy.
pub fn generate_temporary() -> anyhow::Result<String> {
    // The largest multiple of the alphabet's length that fits in a byte:
    // bytes above it are drawn again, so that every character is equally
    // likely.
    let alphabet_len = TEMPORARY_ALPHABET.len();
    let limit = u8::try_from(256 / alphabet_len * alphabet_len).unwrap_or(u8::MAX);
    let mut chars = Vec::with_capacity(TEMPORARY_GROUPS * TEMPORARY_GROUP_CHARS);
    while chars.len() < chars.capacity() {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| anyhow!("no randomness for a password: {e}"))?;
        chars.extend(
            bytes
                .into_iter()
                .filter(|&byte| byte < limit)
                .map(|byte| TEMPORARY_ALPHABET[usize::from(byte) % alphabet_len] as char)
                .take(chars.capacity() - chars.len()),
        );
    }
    Ok(chars
        .chunks(TEMPORARY_GROUP_CHARS)
        .map(|group| group.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-"))
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
    fn temporary_passwords_are_long_unambiguous_and_distinct() {
        let first = super::generate_temporary().unwrap();
        let second = super::generate_temporary().unwrap();
        assert_ne!(first, second);
        assert_eq!(first.len(), 23);
        assert!(super::is_long_enough(&first));
        let groups: Vec<_> = first.split('-').collect();
        assert_eq!(groups.len(), 4);
        for group in groups {
            assert_eq!(group.len(), 5);
            assert!(
                group
                    .bytes()
                    .all(|b| super::TEMPORARY_ALPHABET.contains(&b))
            );
        }
        for ambiguous in ['0', 'o', '1', 'l', 'i'] {
            assert!(!super::TEMPORARY_ALPHABET.contains(&(ambiguous as u8)));
        }
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert!(super::is_long_enough("éééééééééééé"));
        assert!(!super::is_long_enough("ééééééééééé"));
    }
}
