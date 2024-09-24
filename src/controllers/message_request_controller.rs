use chrono::Utc;
use rspc::{Error, ErrorCode};

use crate::{
        dtos::{MessageRequestRequestDto, MessageRequestResponseDto},
        models::{GroupUserWithRelationships, GroupWithRelationships, MessageRequestWithRelationships},
        RequestContext,
};

pub async fn get_message_request(
        ctx: RequestContext,
        message_request_id: String,
) -> Result<MessageRequestResponseDto, Error> {
        let message_request_id: i64 = message_request_id
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid message_request_id".into()))?;

        let auth_user = ctx.get_auth_user().await?;

        let message_request = ctx
                .app_state
                .message_request_repository
                .find_by_id_and_destination_id(message_request_id, auth_user.id)?
                .ok_or(Error::new(ErrorCode::NotFound, "Message request not found".into()))?;

        let message_request_response = MessageRequestResponseDto::from(message_request);

        Ok(message_request_response)
}

pub async fn create_message_request(
        ctx: RequestContext,
        message_request_request: MessageRequestRequestDto,
) -> Result<MessageRequestResponseDto, Error> {
        let auth_user = ctx.get_auth_user().await?;

        let destination_id: i64 = message_request_request
                .destination_id
                .clone()
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid destinationId".into()))?;

        let destination = ctx
                .app_state
                .user_repository
                .find_by_id(destination_id)?
                .ok_or(Error::new(ErrorCode::NotFound, "Destination user not found".into()))?;

        let message_request = {
                let mut id_generator = ctx.app_state.id_generator.lock().unwrap();
                ctx.app_state
                        .message_request_repository
                        .save(MessageRequestWithRelationships {
                                id: id_generator.generate(),
                                created_at: Utc::now().naive_utc(),
                                updated_at: Utc::now().naive_utc(),
                                source: auth_user.clone(),
                                destination: destination.clone(),
                                approved_at: None,
                        })?
        };

        let message_request_response = MessageRequestResponseDto::from(message_request);

        Ok(message_request_response)
}

pub async fn approve_message_request(
        ctx: RequestContext,
        message_request_id: String,
) -> Result<MessageRequestResponseDto, Error> {
        let message_request_id: i64 = message_request_id
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid message_request_id".into()))?;

        let auth_user = ctx.get_auth_user().await?;

        let mut message_request = ctx
                .app_state
                .message_request_repository
                .find_by_id_and_destination_id(message_request_id, auth_user.id)?
                .ok_or(Error::new(ErrorCode::NotFound, "Message request not found".into()))?;

        message_request.updated_at = Utc::now().naive_utc();
        message_request.approved_at = Some(Utc::now().naive_utc());
        let message_request = ctx.app_state.message_request_repository.save(message_request)?;

        {
                let mut id_generator = ctx.app_state.id_generator.lock().unwrap();
                ctx.app_state.group_repository.save(GroupWithRelationships {
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

        let message_request_response = MessageRequestResponseDto::from(message_request);

        Ok(message_request_response)
}
