use std::env;

use aws_sdk_cognitoidentityprovider::Client;
use chrono::Utc;
use rspc::{Error, ErrorCode};
use tracing::error;

use crate::dtos::{UserRequestDto, UserResponseDto};
use crate::models::User;
use crate::AppContext;

pub async fn get_user(ctx: AppContext, user_id: String) -> Result<UserResponseDto, Error> {
        let user_id: i64 = user_id
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid user_id".into()))?;

        let user = ctx
                .user_repository
                .find_by_id(user_id)?
                .ok_or(Error::new(ErrorCode::NotFound, "User not found".into()))?;

        let user_response = UserResponseDto::from(user);

        Ok(user_response)
}

pub async fn create_user(ctx: AppContext, user_request: UserRequestDto) -> Result<UserResponseDto, Error> {
        let sub =
                ctx.sub.lock()
                        .map_err(|_| {
                                error!("failed to retrieve sub from app data");
                                Error::new(
                                        ErrorCode::InternalServerError,
                                        "Failed to retrieve sub from app data".into(),
                                )
                        })?
                        .clone()
                        .ok_or_else(|| {
                                error!("failed to retrieve sub from app data");
                                Error::new(
                                        ErrorCode::InternalServerError,
                                        "Failed to retrieve sub from sapp data".into(),
                                )
                        })?;

        if ctx.user_repository.exists_by_sub(sub.clone().to_string())? {
                return Err(Error::new(ErrorCode::Conflict, "User already exists".into()));
        }

        let user_pool_id = env::var("AWS_COGNITO_USER_POOL_ID").expect("AWS_COGNITO_USER_POOL_ID must be set");
        let shared_config = aws_config::load_from_env().await;
        let client = Client::new(&shared_config);
        let cognito_user = client
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

        let mut email = String::new();
        let mut phone_number = String::new();
        let mut first_name = String::new();
        let mut last_name = String::new();

        for attribute in cognito_user.user_attributes.unwrap_or_default() {
                match attribute.name.as_str() {
                        "email" => email = attribute.value.unwrap_or_default(),
                        "phone_number" => phone_number = attribute.value.unwrap_or_default(),
                        "given_name" => first_name = attribute.value.unwrap_or_default(),
                        "family_name" => last_name = attribute.value.unwrap_or_default(),
                        _ => {}
                }
        }

        let user = {
                let mut id_generator = ctx.id_generator.lock().unwrap();
                ctx.user_repository.save(User {
                        id: id_generator.generate(),
                        created_at: Utc::now().naive_utc(),
                        updated_at: Utc::now().naive_utc(),
                        sub: sub.clone().to_string(),
                        email,
                        phone_number,
                        first_name,
                        last_name,
                        display_name: None,
                        public_key: user_request.public_key.clone(),
                })?
        };

        let user_response = UserResponseDto::from(user);

        Ok(user_response)
}
