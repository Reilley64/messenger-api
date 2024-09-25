use std::env;

use derive_new::new;
use google_cloud_storage::{
        client::Client,
        sign::{SignedURLMethod, SignedURLOptions},
};
use rspc::{Error, ErrorCode};

#[derive(new, Clone)]
pub struct GoogleCloudStorageService {
        client: Client,
}

impl GoogleCloudStorageService {
        pub async fn get_presigned_upload_url(&self, key: String, content_type: String) -> Result<String, Error> {
                let bucket = env::var("GCP_USER_PROFILE_PICTURE_BUCKET")
                        .expect("GCP_USER_PROFILE_PICTURE_BUCKET must be set");

                let presigned_url = self
                        .client
                        .signed_url(
                                bucket.as_str(),
                                key.as_str(),
                                None,
                                None,
                                SignedURLOptions {
                                        content_type: Some(content_type),
                                        method: SignedURLMethod::PUT,
                                        ..Default::default()
                                },
                        )
                        .await
                        .map_err(|_| {
                                Error::new(ErrorCode::InternalServerError, "Failed to get presigned request".into())
                        })?;

                Ok(presigned_url)
        }
}
