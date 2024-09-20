use actix_web::web::{Data, Json};
use apistos::api_operation;

use crate::dtos::{GroupResponseDto, MessageWithGroupResponseDto, UserResponseDto};
use crate::errors::problem::Problem;
use crate::{get_auth_user_from_cache, AppState, BearerAuth};

#[api_operation(operation_id = "get_messages")]
pub async fn get_messages(
        _: BearerAuth,
        data: Data<AppState>,
) -> Result<Json<Vec<MessageWithGroupResponseDto>>, Problem> {
        let auth_user = get_auth_user_from_cache(&data).await?;

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
