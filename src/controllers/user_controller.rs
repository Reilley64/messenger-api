use actix_web::web::{Data, Json, Path};
use apistos::api_operation;
use aws_sdk_cognitoidentityprovider::Client;
use chrono::Utc;
use log::error;
use std::env;

use crate::dtos::{UserRequestDto, UserResponseDto};
use crate::errors::problem::Problem;
use crate::models::User;
use crate::{AppState, BearerAuth};

#[api_operation(operation_id = "get_user")]
pub async fn get_user(
        _: BearerAuth,
        data: Data<AppState>,
        path: Path<(String,)>,
) -> Result<Json<UserResponseDto>, Problem> {
        let user_id = path
                .into_inner()
                .0
                .parse::<i64>()
                .map_err(|_| Problem::BadRequest("Invalid user_id".to_string()))?;

        let user = data
                .user_repository
                .find_by_id(user_id)?
                .ok_or(Problem::NotFound("User not found".to_string()))?;

        Ok(Json(UserResponseDto::from(user)))
}

#[api_operation(operation_id = "create_user")]
pub async fn create_user(
        _: BearerAuth,
        data: Data<AppState>,
        body: Json<UserRequestDto>,
) -> Result<Json<UserResponseDto>, Problem> {
        let sub =
                data.sub.lock()
                        .map_err(|_| {
                                error!("failed to retrieve sub from app data");
                                Problem::InternalServerError("failed to retrieve sub from  app data".to_string())
                        })?
                        .clone()
                        .ok_or_else(|| {
                                error!("failed to retrieve sub from app data");
                                Problem::InternalServerError("failed to retrieve sub from  app data".to_string())
                        })?;

        if data.user_repository.exists_by_sub(sub.clone().to_string())? {
                return Err(Problem::Conflict("User already exists".to_string()));
        }

        let user_pool_id = env::var("AWS_COGNITO_USER_POOL_ID").expect("AWS_COGNITO_USER_POOL_ID must be set");

        let shared_config = aws_config::load_from_env().await;
        let client = Client::new(&shared_config);
        let cognito_user = match client
                .admin_get_user()
                .user_pool_id(user_pool_id)
                .username(sub.clone())
                .send()
                .await
        {
                Ok(cognito_user) => cognito_user,
                Err(_) => {
                        return Err(Problem::InternalServerError(
                                "failed to retrieve user from cognito".to_string(),
                        ))
                }
        };

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
                let mut id_generator = data.id_generator.lock().unwrap();
                data.user_repository.save(User {
                        id: id_generator.generate(),
                        created_at: Utc::now().naive_utc(),
                        updated_at: Utc::now().naive_utc(),
                        sub: sub.clone().to_string(),
                        email,
                        phone_number,
                        first_name,
                        last_name,
                        display_name: None,
                        public_key: body.public_key.clone(),
                })?
        };

        Ok(Json(UserResponseDto::from(user)))
}
