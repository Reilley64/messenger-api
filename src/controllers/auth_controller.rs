use actix_web::web::{Data, Json};
use apistos::api_operation;

use crate::{dtos::UserResponseDto, errors::problem::Problem, get_auth_user_from_cache, AppState, BearerAuth};

#[api_operation(operation_id = "get_auth_user")]
pub async fn get_auth_user(_: BearerAuth, data: Data<AppState>) -> Result<Json<UserResponseDto>, Problem> {
        let auth_user = get_auth_user_from_cache(&data).await?;

        Ok(Json(UserResponseDto::from(auth_user)))
}
