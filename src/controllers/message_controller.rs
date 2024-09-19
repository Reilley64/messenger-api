use actix_web::web::{Data, Json};
use apistos::api_operation;
use log::error;

use crate::dtos::{GroupResponseDto, MessageWithGroupResponseDto, UserResponseDto};
use crate::errors::problem::Problem;
use crate::{AppState, BearerAuth};

#[api_operation(operation_id = "get_messages")]
pub async fn get_messages(
        _: BearerAuth,
        data: Data<AppState>,
) -> Result<Json<Vec<MessageWithGroupResponseDto>>, Problem> {
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

        let auth_user = data.user_repository.find_by_sub(sub)?.ok_or_else(|| {
                error!("failed to find auth user with sub");
                Problem::InternalServerError("failed to find auth user with sub".to_string())
        })?;

        let messages = data.message_repositoy.find_by_user_id(auth_user.id)?;

        let message_responses = messages
                .into_iter()
                .map(|message| MessageWithGroupResponseDto {
                        id: message.id.to_string(),
                        created_at: message.created_at,
                        updated_at: message.updated_at,
                        group: GroupResponseDto::from(message.group),
                        source: UserResponseDto::from(message.source),
                        content: message.content,
                        idempotency_key: message.idempotency_key,
                })
                .collect::<Vec<MessageWithGroupResponseDto>>();

        Ok(Json(message_responses))
}
