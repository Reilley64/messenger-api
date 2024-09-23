use std::env;
use std::time::Duration;

use aws_sdk_s3::{presigning::PresigningConfig, Client};
use derive_new::new;
use rspc::{Error, ErrorCode};

#[derive(new, Debug, Clone)]
pub struct S3Service {
        client: Client,
}

impl S3Service {
        pub async fn get_presigned_upload_url(&self, user_id: i64) -> Result<String, Error> {
                let bucket = env::var("AWS_S3_USER_PROFILE_PICTURE_BUCKET")
                        .expect("AWS_S3_USER_PROFILE_PICTURE_BUCKET_ID must be set");

                let presigning_config =
                        PresigningConfig::expires_in(Duration::from_secs(60 * 60 * 24 * 7)).map_err(|_| {
                                Error::new(
                                        ErrorCode::InternalServerError,
                                        "Failed to convert expiration to PresigningConfig".into(),
                                )
                        })?;

                let presigned_request = self
                        .client
                        .put_object()
                        .bucket(bucket)
                        .key(user_id.to_string())
                        .presigned(presigning_config)
                        .await
                        .map_err(|_| {
                                Error::new(ErrorCode::InternalServerError, "Failed to get presigned request".into())
                        })?;

                let presigned_url = presigned_request.uri().to_string();

                Ok(presigned_url)
        }
}
