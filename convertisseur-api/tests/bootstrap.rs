use argon2::{Argon2, PasswordVerifier};
use convertisseur_api::ensure_super_admin;
use sqlx::PgPool;
use uuid::Uuid;

const PASSWORD: &str = "a long enough password";

struct UserRow {
    id: Uuid,
    username: String,
    role: String,
    password_hash: String,
}

async fn users(pool: &PgPool) -> Vec<UserRow> {
    sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT id, username, role::text, password_hash FROM users ORDER BY username",
    )
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|(id, username, role, password_hash)| UserRow {
        id,
        username,
        role,
        password_hash,
    })
    .collect()
}

fn verifies(password: &str, hash: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), hash)
        .is_ok()
}

#[sqlx::test]
async fn creates_the_super_admin(pool: PgPool) {
    let id = ensure_super_admin(&pool, "  admin ", PASSWORD.to_owned())
        .await
        .unwrap();

    let users = users(&pool).await;
    assert_eq!(users.len(), 1);
    let admin = &users[0];
    assert_eq!(admin.id, id);
    assert_eq!(admin.username, "admin");
    assert_eq!(admin.role, "super_admin");
    assert!(admin.password_hash.starts_with("$argon2id$"));
    assert!(verifies(PASSWORD, &admin.password_hash));
}

#[sqlx::test]
async fn updates_the_password(pool: PgPool) {
    let first = ensure_super_admin(&pool, "admin", PASSWORD.to_owned())
        .await
        .unwrap();
    let second = ensure_super_admin(&pool, "admin", "another long password".to_owned())
        .await
        .unwrap();

    assert_eq!(first, second);
    let users = users(&pool).await;
    assert_eq!(users.len(), 1);
    assert!(verifies("another long password", &users[0].password_hash));
    assert!(!verifies(PASSWORD, &users[0].password_hash));
}

#[sqlx::test]
async fn demotes_another_super_admin(pool: PgPool) {
    ensure_super_admin(&pool, "alice", PASSWORD.to_owned())
        .await
        .unwrap();
    ensure_super_admin(&pool, "bob", PASSWORD.to_owned())
        .await
        .unwrap();

    let roles: Vec<_> = users(&pool)
        .await
        .into_iter()
        .map(|u| (u.username, u.role))
        .collect();
    assert_eq!(
        roles,
        vec![
            ("alice".to_owned(), "admin".to_owned()),
            ("bob".to_owned(), "super_admin".to_owned()),
        ]
    );
}

#[sqlx::test]
async fn promotes_an_existing_account(pool: PgPool) {
    sqlx::query("INSERT INTO users (username, password_hash) VALUES ('carol', 'x'), ('dave', 'x')")
        .execute(&pool)
        .await
        .unwrap();

    ensure_super_admin(&pool, "Carol", PASSWORD.to_owned())
        .await
        .unwrap();

    let users = users(&pool).await;
    let roles: Vec<_> = users
        .iter()
        .map(|u| (u.username.as_str(), u.role.as_str()))
        .collect();
    assert_eq!(roles, vec![("carol", "super_admin"), ("dave", "user")]);
    assert!(verifies(PASSWORD, &users[0].password_hash));
}

#[sqlx::test]
async fn is_idempotent(pool: PgPool) {
    let first = ensure_super_admin(&pool, "admin", PASSWORD.to_owned())
        .await
        .unwrap();
    let second = ensure_super_admin(&pool, "ADMIN", PASSWORD.to_owned())
        .await
        .unwrap();

    assert_eq!(first, second);
    let users = users(&pool).await;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].username, "admin");
    assert_eq!(users[0].role, "super_admin");
}

#[sqlx::test]
async fn rejects_a_blank_username_or_a_short_password(pool: PgPool) {
    assert!(
        ensure_super_admin(&pool, "  ", PASSWORD.to_owned())
            .await
            .is_err()
    );
    assert!(
        ensure_super_admin(&pool, "admin", "too short".to_owned())
            .await
            .is_err()
    );
    assert!(users(&pool).await.is_empty());
}

#[sqlx::test]
async fn the_schema_allows_a_single_super_admin(pool: PgPool) {
    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, role) \
         VALUES ('erin', 'x', 'super_admin'), ('frank', 'x', 'super_admin')",
    )
    .execute(&pool)
    .await;

    assert!(
        result
            .unwrap_err()
            .as_database_error()
            .unwrap()
            .is_unique_violation()
    );
}
