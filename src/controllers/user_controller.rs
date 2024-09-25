use chrono::Utc;
use rspc::{Error, ErrorCode};

use crate::{
        dtos::{PresignedUploadUrlRequestDto, PresignedUploadUrlResponseDto, UserRequestDto, UserResponseDto},
        models::User,
        RequestContext,
};

pub async fn get_user(ctx: RequestContext, user_id: String) -> Result<UserResponseDto, Error> {
        let user_id: i64 = user_id
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid user_id".into()))?;

        let user = ctx
                .app_state
                .user_repository
                .find_by_id(user_id)?
                .ok_or(Error::new(ErrorCode::NotFound, "User not found".into()))?;

        let user_response = UserResponseDto::from(user);

        Ok(user_response)
}

pub async fn create_user(ctx: RequestContext, user_request: UserRequestDto) -> Result<UserResponseDto, Error> {
        let sub = ctx.sub.ok_or_else(|| {
                tracing::error!("failed to retrieve sub from app data");
                Error::new(
                        ErrorCode::InternalServerError,
                        "Failed to retrieve sub from sapp data".into(),
                )
        })?;

        if ctx.app_state.user_repository.exists_by_sub(sub.clone().to_string())? {
                return Err(Error::new(ErrorCode::Conflict, "User already exists".into()));
        }

        let user = {
                let mut id_generator = ctx.app_state.id_generator.lock().unwrap();
                ctx.app_state.user_repository.save(User {
                        id: id_generator.generate(),
                        created_at: Utc::now().naive_utc(),
                        updated_at: Utc::now().naive_utc(),
                        sub: sub.clone().to_string(),
                        email: user_request.email.clone(),
                        first_name: user_request.first_name.clone(),
                        last_name: user_request.last_name.clone(),
                        display_name: None,
                        public_key: user_request.public_key.clone(),
                })?
        };

        let user_response = UserResponseDto::from(user);

        Ok(user_response)
}

pub async fn create_user_profile_picture_presigned_upload_url(
        ctx: RequestContext,
        presigned_upload_url_request: PresignedUploadUrlRequestDto,
) -> Result<PresignedUploadUrlResponseDto, Error> {
        let auth_user = ctx.get_auth_user().await?;

        let presigned_url = ctx
                .app_state
                .google_cloud_storage_service
                .get_presigned_upload_url(format!("u/{}", auth_user.id), presigned_upload_url_request.content_type)
                .await?;

        let presigned_url_response = PresignedUploadUrlResponseDto { url: presigned_url };

        Ok(presigned_url_response)
}
