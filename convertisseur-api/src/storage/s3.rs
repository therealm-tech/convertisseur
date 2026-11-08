use std::{ops::RangeInclusive, time::Duration};

use anyhow::Context;
use aws_sdk_s3::{
    Client,
    config::{BehaviorVersion, Credentials, Region, timeout::TimeoutConfig},
    error::DisplayErrorContext,
    operation::{get_object::GetObjectError, head_bucket::HeadBucketError},
    primitives::ByteStream,
    types::{BucketLocationConstraint, CreateBucketConfiguration},
};
use aws_smithy_http_client::{
    Builder,
    tls::{self, rustls_provider::CryptoMode},
};
use axum::body::Body;
use bytes::Bytes;

use super::ObjectStore;
use crate::config::StorageArgs;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// The SDK's errors display as a bare "service error"; this keeps the cause.
fn explain(error: impl std::error::Error) -> anyhow::Error {
    anyhow::anyhow!("{}", DisplayErrorContext(error))
}

/// An S3-compatible object store, holding every object in one bucket.
#[derive(Clone)]
pub struct S3Store {
    client: Client,
    bucket: String,
}

impl S3Store {
    /// A client for the store `args` describes. It connects on its first
    /// request.
    #[must_use]
    pub fn new(args: &StorageArgs) -> Self {
        let credentials = Credentials::new(
            &args.access_key_id,
            &args.secret_access_key,
            None,
            None,
            "configuration",
        );
        let http_client = Builder::new()
            .tls_provider(tls::Provider::Rustls(CryptoMode::Ring))
            .build_https();
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .http_client(http_client)
            .endpoint_url(&args.endpoint)
            .region(Region::new(args.region.clone()))
            .credentials_provider(credentials)
            .force_path_style(args.force_path_style)
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(CONNECT_TIMEOUT)
                    .build(),
            )
            .build();
        Self {
            client: Client::from_conf(config),
            bucket: args.bucket.clone(),
        }
    }

    #[must_use]
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Creates the bucket when it does not exist. A bucket that exists needs
    /// no permission beyond reading it.
    ///
    /// # Errors
    ///
    /// Fails when the store does not answer, or refuses to show or create the
    /// bucket.
    pub async fn ensure_bucket(&self) -> anyhow::Result<()> {
        let head = self.client.head_bucket().bucket(&self.bucket).send().await;
        match head {
            Ok(_) => {
                tracing::debug!(bucket = self.bucket, "bucket exists");
                return Ok(());
            }
            Err(error)
                if error
                    .as_service_error()
                    .is_some_and(HeadBucketError::is_not_found) => {}
            Err(error) => {
                return Err(explain(error))
                    .with_context(|| format!("failed to look up bucket {}", self.bucket));
            }
        }

        let mut create = self.client.create_bucket().bucket(&self.bucket);
        // us-east-1 is the default location, which S3 refuses to be named.
        if let Some(region) = self.client.config().region()
            && region.as_ref() != "us-east-1"
        {
            create = create.create_bucket_configuration(
                CreateBucketConfiguration::builder()
                    .location_constraint(BucketLocationConstraint::from(region.as_ref()))
                    .build(),
            );
        }
        match create.send().await {
            Ok(_) => {
                tracing::info!(bucket = self.bucket, "bucket created");
                Ok(())
            }
            Err(error)
                if error.as_service_error().is_some_and(|e| {
                    e.is_bucket_already_owned_by_you() || e.is_bucket_already_exists()
                }) =>
            {
                tracing::debug!(bucket = self.bucket, "bucket created concurrently");
                Ok(())
            }
            Err(error) => Err(explain(error))
                .with_context(|| format!("failed to create bucket {}", self.bucket)),
        }
    }

    /// Deletes every object in the bucket, then the bucket.
    #[cfg(test)]
    pub(super) async fn destroy_bucket(&self) -> anyhow::Result<()> {
        let listing = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .send()
            .await?;
        for object in listing.contents() {
            if let Some(key) = object.key() {
                self.delete(key).await?;
            }
        }
        self.client
            .delete_bucket()
            .bucket(&self.bucket)
            .send()
            .await?;
        Ok(())
    }
}

impl ObjectStore for S3Store {
    async fn check(&self) -> anyhow::Result<()> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map_err(explain)?;
        Ok(())
    }

    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> anyhow::Result<()> {
        let size = body.len();
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(body))
            .send()
            .await
            .map_err(explain)
            .with_context(|| format!("failed to store object {key}"))?;
        tracing::debug!(key, size, content_type, "object stored");
        Ok(())
    }

    async fn get(
        &self,
        key: &str,
        range: Option<RangeInclusive<u64>>,
    ) -> anyhow::Result<Option<Body>> {
        let mut request = self.client.get_object().bucket(&self.bucket).key(key);
        if let Some(range) = &range {
            request = request.range(format!("bytes={}-{}", range.start(), range.end()));
        }
        match request.send().await {
            Ok(output) => {
                tracing::debug!(key, ?range, "object read");
                Ok(Some(Body::new(output.body.into_inner())))
            }
            Err(error)
                if error
                    .as_service_error()
                    .is_some_and(GetObjectError::is_no_such_key) =>
            {
                tracing::debug!(key, "no such object");
                Ok(None)
            }
            Err(error) => {
                Err(explain(error)).with_context(|| format!("failed to read object {key}"))
            }
        }
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(explain)
            .with_context(|| format!("failed to delete object {key}"))?;
        tracing::debug!(key, "object deleted");
        Ok(())
    }
}
