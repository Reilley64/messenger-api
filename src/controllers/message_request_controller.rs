use actix_web::web::{Data, Json, Path};
use apistos::api_operation;
use chrono::Utc;

use crate::dtos::{MessageRequestRequestDto, MessageRequestResponseDto};
use crate::errors::problem::Problem;
use crate::models::{GroupUserWithRelationships, GroupWithRelationships, MessageRequestWithRelationships};
use crate::{get_auth_user_from_cache, AppState, BearerAuth};

#[api_operation(operation_id = "get_message_request")]
pub async fn get_message_request(
        _: BearerAuth,
        data: Data<AppState>,
        path: Path<(String,)>,
) -> Result<Json<MessageRequestResponseDto>, Problem> {
        let message_request_id = path
                .into_inner()
                .0
                .parse::<i64>()
                .map_err(|_| Problem::BadRequest("Invalid message_request_id".to_string()))?;

        let auth_user = get_auth_user_from_cache(&data).await?;

        let message_request = data
                .message_request_repository
                .find_by_id_and_destination_id(message_request_id, auth_user.id)?
                .ok_or(Problem::NotFound("Message request not found".to_string()))?;

        Ok(Json(MessageRequestResponseDto::from(message_request)))
}

#[api_operation(operation_id = "create_message_request")]
pub async fn create_message_request(
        _: BearerAuth,
        data: Data<AppState>,
        body: Json<MessageRequestRequestDto>,
) -> Result<Json<MessageRequestResponseDto>, Problem> {
        let auth_user = get_auth_user_from_cache(&data).await?;

        let destination_id = body
                .destination_id
                .clone()
                .parse::<i64>()
                .map_err(|_| Problem::BadRequest("Invalid destinationId".to_string()))?;

        let destination = data
                .user_repository
                .find_by_id(destination_id)?
                .ok_or(Problem::NotFound("Destination user not found".to_string()))?;

        let message_request = {
                let mut id_generator = data.id_generator.lock().unwrap();
                data.message_request_repository.save(MessageRequestWithRelationships {
                        id: id_generator.generate(),
                        created_at: Utc::now().naive_utc(),
                        updated_at: Utc::now().naive_utc(),
                        source: auth_user.clone(),
                        destination: destination.clone(),
                        approved_at: None,
                })?
        };

        Ok(Json(MessageRequestResponseDto::from(message_request)))
}

#[api_operation(operation_id = "approve_message_request")]
pub async fn approve_message_request(
        _: BearerAuth,
        data: Data<AppState>,
        path: Path<(String,)>,
) -> Result<Json<MessageRequestResponseDto>, Problem> {
        let message_request_id = path
                .into_inner()
                .0
                .parse::<i64>()
                .map_err(|_| Problem::BadRequest("Invalid message_request_id".to_string()))?;

        let auth_user = get_auth_user_from_cache(&data).await?;

        let mut message_request = data
                .message_request_repository
                .find_by_id_and_destination_id(message_request_id, auth_user.id)?
                .ok_or(Problem::NotFound("Message request found".to_string()))?;

        message_request.updated_at = Utc::now().naive_utc();
        message_request.approved_at = Some(Utc::now().naive_utc());
        let message_request = data.message_request_repository.save(message_request)?;

        {
                let mut id_generator = data.id_generator.lock().unwrap();
                data.group_repository.save(GroupWithRelationships {
                        id: id_generator.generate(),
                        created_at: Utc::now().naive_utc(),
                        updated_at: Utc::now().naive_utc(),
                        name: None,
                        message_request_id: Some(message_request.id),
                        users: vec![
                                GroupUserWithRelationships {
                                        id: id_generator.generate(),
                                        created_at: Utc::now().naive_utc(),
                                        updated_at: Utc::now().naive_utc(),
                                        user: message_request.source.clone(),
                                        is_admin: true,
                                        nickname: None,
                                },
                                GroupUserWithRelationships {
                                        id: id_generator.generate(),
                                        created_at: Utc::now().naive_utc(),
                                        updated_at: Utc::now().naive_utc(),
                                        user: message_request.destination.clone(),
                                        is_admin: true,
                                        nickname: None,
                                },
                        ],
                })?;
        }

        Ok(Json(MessageRequestResponseDto::from(message_request)))
}
