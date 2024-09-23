use std::env;

use aws_sdk_cognitoidentityprovider::{operation::admin_get_user::AdminGetUserOutput, Client};
use derive_new::new;
use rspc::{Error, ErrorCode};

#[derive(new, Debug, Clone)]
pub struct CognitoService {
        client: Client,
}

impl CognitoService {
        pub async fn get_cognito_user(&self, sub: String) -> Result<AdminGetUserOutput, Error> {
                let user_pool_id = env::var("AWS_COGNITO_USER_POOL_ID").expect("AWS_COGNITO_USER_POOL_ID must be set");

                let cognito_user = self
                        .client
                        .admin_get_user()
                        .user_pool_id(user_pool_id)
                        .username(sub.clone())
                        .send()
                        .await
                        .map_err(|_| {
                                Error::new(
                                        ErrorCode::InternalServerError,
                                        "Failed to retrieve user from cognito".into(),
                                )
                        })?;

                Ok(cognito_user)
        }
}
