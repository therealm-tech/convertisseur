//! The object store holding user-uploaded media. The server talks to it
//! through [`ObjectStore`]: [`S3Store`] in production, [`MemoryStore`] in
//! tests that do not exercise the S3 protocol itself.

mod memory;
mod s3;

use std::ops::RangeInclusive;

use axum::body::Body;
use bytes::Bytes;

pub use memory::MemoryStore;
pub use s3::S3Store;

/// What the server needs from an object store. Keys are the server's own,
/// generated ones.
pub trait ObjectStore {
    /// Succeeds when the store answers and its bucket exists.
    fn check(&self) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Stores `body` under `key`, replacing any object already there.
    fn put(
        &self,
        key: &str,
        content_type: &str,
        body: Bytes,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// The object's bytes, or those in `range`, which must start inside the
    /// object and is cut at its end; `None` when no object has this key.
    fn get(
        &self,
        key: &str,
        range: Option<RangeInclusive<u64>>,
    ) -> impl Future<Output = anyhow::Result<Option<Body>>> + Send;

    /// Deletes the object; succeeds when there is none.
    fn delete(&self, key: &str) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// The store the server runs with.
#[derive(Clone)]
pub enum Storage {
    S3(S3Store),
    Memory(MemoryStore),
}

impl ObjectStore for Storage {
    async fn check(&self) -> anyhow::Result<()> {
        match self {
            Self::S3(store) => store.check().await,
            Self::Memory(store) => store.check().await,
        }
    }

    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> anyhow::Result<()> {
        match self {
            Self::S3(store) => store.put(key, content_type, body).await,
            Self::Memory(store) => store.put(key, content_type, body).await,
        }
    }

    async fn get(
        &self,
        key: &str,
        range: Option<RangeInclusive<u64>>,
    ) -> anyhow::Result<Option<Body>> {
        match self {
            Self::S3(store) => store.get(key, range).await,
            Self::Memory(store) => store.get(key, range).await,
        }
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        match self {
            Self::S3(store) => store.delete(key).await,
            Self::Memory(store) => store.delete(key).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use bytes::Bytes;
    use clap::Parser;

    use super::{MemoryStore, ObjectStore, S3Store};
    use crate::config::StorageArgs;

    async fn read(store: &impl ObjectStore, key: &str, range: Option<(u64, u64)>) -> Option<Bytes> {
        let body = store
            .get(key, range.map(|(first, last)| first..=last))
            .await
            .unwrap()?;
        Some(to_bytes(body, usize::MAX).await.unwrap())
    }

    /// What every implementation must do the same way.
    async fn honours_the_contract(store: &impl ObjectStore) {
        store.check().await.unwrap();
        let key = "audio/a8d1";
        assert_eq!(read(store, key, None).await, None);

        store
            .put(key, "audio/ogg", Bytes::from_static(b"OggS0123456789"))
            .await
            .unwrap();
        assert_eq!(
            read(store, key, None).await.as_deref(),
            Some(&b"OggS0123456789"[..])
        );
        assert_eq!(
            read(store, key, Some((4, 7))).await.as_deref(),
            Some(&b"0123"[..])
        );
        assert_eq!(
            read(store, key, Some((10, 99))).await.as_deref(),
            Some(&b"6789"[..])
        );

        store
            .put(key, "audio/ogg", Bytes::from_static(b"OggS"))
            .await
            .unwrap();
        assert_eq!(read(store, key, None).await.as_deref(), Some(&b"OggS"[..]));

        store.delete(key).await.unwrap();
        assert_eq!(read(store, key, None).await, None);
        store.delete(key).await.unwrap();
    }

    #[tokio::test]
    async fn the_memory_store_honours_the_contract() {
        honours_the_contract(&MemoryStore::default()).await;
    }

    #[derive(Parser)]
    struct TestConfig {
        #[command(flatten)]
        storage: StorageArgs,
    }

    /// Runs against the store `S3_ENDPOINT` names, in a bucket of its own
    /// that it creates, then empties and deletes.
    #[tokio::test]
    async fn the_s3_store_honours_the_contract() {
        let mut args = TestConfig::parse_from(["test"]).storage;
        let mut suffix = [0_u8; 8];
        getrandom::fill(&mut suffix).unwrap();
        args.bucket = format!("test-{}", u64::from_be_bytes(suffix));
        let store = S3Store::new(&args);
        assert!(store.check().await.is_err(), "the bucket exists already");
        store.ensure_bucket().await.unwrap();
        store.ensure_bucket().await.unwrap();

        let tested = store.clone();
        let outcome = tokio::spawn(async move { honours_the_contract(&tested).await }).await;
        store.destroy_bucket().await.unwrap();
        outcome.unwrap();
    }
}
