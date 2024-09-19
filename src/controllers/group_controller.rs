use actix_web::web::{Data, Json, Path};
use apistos::api_operation;
use chrono::Utc;
use log::error;

use crate::dtos::{GroupResponseDto, MessageRequestDto, MessageResponseDto, UserResponseDto};
use crate::errors::problem::Problem;
use crate::models::MessageWithSource;
use crate::{AppState, BearerAuth};

#[api_operation(operation_id = "get_group")]
pub async fn get_group(
        _: BearerAuth,
        data: Data<AppState>,
        path: Path<(String,)>,
) -> Result<Json<GroupResponseDto>, Problem> {
        let group_id = path
                .into_inner()
                .0
                .parse::<i64>()
                .map_err(|_| Problem::BadRequest("Invalid group_id".to_string()))?;

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

        let group = data
                .group_repository
                .find_by_id_and_user_id(group_id, auth_user.id)?
                .ok_or(Problem::NotFound("Group not found".to_string()))?;

        Ok(Json(GroupResponseDto::from(group)))
}

#[api_operation(operation_id = "get_group_messages")]
pub async fn get_group_messages(
        _: BearerAuth,
        data: Data<AppState>,
        path: Path<(String,)>,
) -> Result<Json<Vec<MessageResponseDto>>, Problem> {
        let group_id = path
                .into_inner()
                .0
                .parse::<i64>()
                .map_err(|_| Problem::BadRequest("Invalid group_id".to_string()))?;

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

        let group = data
                .group_repository
                .find_by_id_and_user_id(group_id, auth_user.id)?
                .ok_or(Problem::NotFound("Group not found".to_string()))?;

        let messages = data.message_repositoy.find_by_group_id(group.id)?;

        let message_responses = messages
                .into_iter()
                .map(|message| MessageResponseDto {
                        id: message.id.to_string(),
                        created_at: message.created_at,
                        updated_at: message.updated_at,
                        source: UserResponseDto::from(message.source),
                        content: message
                                .content
                                .get(&auth_user.id)
                                .expect("Got message not for auth user")
                                .clone(),
                        idempotency_key: message.idempotency_key,
                })
                .collect::<Vec<MessageResponseDto>>();

        Ok(Json(message_responses))
}

#[api_operation(operation_id = "create_group_message")]
pub async fn create_group_message(
        _: BearerAuth,
        data: Data<AppState>,
        path: Path<(String,)>,
        body: Json<MessageRequestDto>,
) -> Result<Json<MessageResponseDto>, Problem> {
        let group_id = path
                .into_inner()
                .0
                .parse::<i64>()
                .map_err(|_| Problem::BadRequest("Invalid group_id".to_string()))?;

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

        let message = {
                let mut id_generator = data.id_generator.lock().unwrap();
                data.message_repositoy.save(MessageWithSource {
                        id: id_generator.generate(),
                        created_at: Utc::now().naive_utc(),
                        updated_at: Utc::now().naive_utc(),
                        group_id,
                        source: auth_user.clone(),
                        content: body
                                .content
                                .clone()
                                .into_iter()
                                .filter_map(|(k, v)| k.parse::<i64>().ok().map(|key| (key, v)))
                                .collect(),
                        idempotency_key: body.idempotency_key.clone(),
                })?
        };

        Ok(Json(MessageResponseDto {
                id: message.id.to_string(),
                created_at: message.created_at,
                updated_at: message.updated_at,
                source: UserResponseDto::from(message.source),
                content: message
                        .content
                        .get(&auth_user.id)
                        .expect("Got message not for auth user")
                        .clone(),
                idempotency_key: message.idempotency_key,
        }))
}
