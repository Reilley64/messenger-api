use rspc::Error;

use crate::{
        dtos::{GroupResponseDto, MessageWithGroupResponseDto, UserResponseDto},
        AppContext,
};

pub async fn get_messages(ctx: AppContext) -> Result<Vec<MessageWithGroupResponseDto>, Error> {
        let auth_user = ctx.get_auth_user().await?;

        let messages = ctx.message_repository.find_by_user_id(auth_user.id)?;

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

        Ok(message_responses)
}
