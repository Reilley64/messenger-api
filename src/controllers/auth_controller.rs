use actix_web::web::{Data, Json};
use apistos::api_operation;
use log::error;

use crate::{dtos::UserResponseDto, errors::problem::Problem, AppState, BearerAuth};

#[api_operation(operation_id = "get_auth_user")]
pub async fn get_auth_user(_: BearerAuth, data: Data<AppState>) -> Result<Json<UserResponseDto>, Problem> {
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

        let auth_user = data
                .user_repository
                .find_by_sub(sub)?
                .ok_or(Problem::NotFound("User not found".to_string()))?;

        Ok(Json(UserResponseDto::from(auth_user)))
}
