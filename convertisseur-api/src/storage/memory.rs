use std::{
    collections::HashMap,
    future::ready,
    ops::RangeInclusive,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use anyhow::ensure;
use axum::body::Body;
use bytes::Bytes;

use super::ObjectStore;

/// An object store in the process's memory, for tests.
#[derive(Clone, Default)]
pub struct MemoryStore {
    objects: Arc<Mutex<HashMap<String, Bytes>>>,
}

impl MemoryStore {
    /// The keys of the stored objects, sorted.
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.objects().keys().cloned().collect();
        keys.sort();
        keys
    }

    fn objects(&self) -> MutexGuard<'_, HashMap<String, Bytes>> {
        self.objects.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn read(&self, key: &str, range: Option<RangeInclusive<u64>>) -> anyhow::Result<Option<Body>> {
        let Some(bytes) = self.objects().get(key).cloned() else {
            return Ok(None);
        };
        let Some(range) = range else {
            return Ok(Some(Body::from(bytes)));
        };
        let len = bytes.len();
        let first = usize::try_from(*range.start()).unwrap_or(usize::MAX);
        let last = usize::try_from(*range.end()).unwrap_or(usize::MAX);
        ensure!(first < len, "range starts past the object's {len} bytes");
        Ok(Some(Body::from(bytes.slice(first..=last.min(len - 1)))))
    }
}

impl ObjectStore for MemoryStore {
    fn check(&self) -> impl Future<Output = anyhow::Result<()>> + Send {
        ready(Ok(()))
    }

    fn put(
        &self,
        key: &str,
        _content_type: &str,
        body: Bytes,
    ) -> impl Future<Output = anyhow::Result<()>> + Send {
        self.objects().insert(key.to_owned(), body);
        ready(Ok(()))
    }

    fn get(
        &self,
        key: &str,
        range: Option<RangeInclusive<u64>>,
    ) -> impl Future<Output = anyhow::Result<Option<Body>>> + Send {
        ready(self.read(key, range))
    }

    fn delete(&self, key: &str) -> impl Future<Output = anyhow::Result<()>> + Send {
        self.objects().remove(key);
        ready(Ok(()))
    }
}
