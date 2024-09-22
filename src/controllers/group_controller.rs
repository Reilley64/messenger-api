use std::env;

use chrono::Utc;
use rspc::{Error, ErrorCode};
use tracing::{error, info};
use web_push::{
        ContentEncoding, SubscriptionInfo, VapidSignatureBuilder, WebPushClient, WebPushMessageBuilder, URL_SAFE_NO_PAD,
};

use crate::{
        dtos::{GroupResponseDto, MessageRequestDto, MessageResponseDto, UserResponseDto},
        models::{GroupWithRelationships, MessageWithSource},
        AppContext,
};

pub async fn get_group(ctx: AppContext, group_id: String) -> Result<GroupResponseDto, Error> {
        let group_id: i64 = group_id
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid group_id".into()))?;

        let auth_user = ctx.get_auth_user().await?;

        let group = ctx
                .group_repository
                .find_by_id_and_user_id(group_id, auth_user.id)?
                .ok_or(Error::new(ErrorCode::NotFound, "Group not found".into()))?;

        let group_response = GroupResponseDto::from(group);

        Ok(group_response)
}

pub async fn get_group_messages(ctx: AppContext, group_id: String) -> Result<Vec<MessageResponseDto>, Error> {
        let group_id: i64 = group_id
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid group_id".into()))?;

        let auth_user = ctx.get_auth_user().await?;

        let group = ctx
                .group_repository
                .find_by_id_and_user_id(group_id, auth_user.id)?
                .ok_or(Error::new(ErrorCode::NotFound, "Group not found".into()))?;

        let messages = ctx.message_repository.find_by_group_id(group.id)?;

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

        Ok(message_responses)
}

async fn send_web_push_notifications(
        ctx: &AppContext,
        group: GroupWithRelationships,
        message_response: MessageResponseDto,
) {
        let push_private_key = env::var("PUSH_PRIVATE_KEY").expect("PUSH_PRIVATE_KET not set");

        for gu in group.users.iter() {
                let user_push_subscription = match ctx
                        .user_push_subscription_repository
                        .find_by_user_id_order_by_created_at_desc(gu.user.id)
                {
                        Ok(Some(subscription)) => subscription,
                        Ok(None) => continue, // No subscription for this user
                        Err(e) => {
                                error!("Error fetching user push subscription: {:?}", e);
                                continue;
                        }
                };

                let subscription_info = SubscriptionInfo::new(
                        user_push_subscription.endpoint,
                        user_push_subscription.p256dh,
                        user_push_subscription.auth,
                );

                let signature_builder = match VapidSignatureBuilder::from_base64(
                        &push_private_key,
                        URL_SAFE_NO_PAD,
                        &subscription_info,
                ) {
                        Ok(builder) => builder,
                        Err(e) => {
                                error!("Failed to build vapid signature: {:?}", e);
                                continue;
                        }
                };

                let signature = match signature_builder.build() {
                        Ok(sig) => sig,
                        Err(e) => {
                                error!("Failed to build vapid signature: {:?}", e);
                                continue;
                        }
                };

                let json_message_response = match serde_json::to_string(&message_response) {
                        Ok(json) => json,
                        Err(e) => {
                                error!("Failed to serialize message response: {:?}", e);
                                continue;
                        }
                };

                let mut web_push_message_build = match WebPushMessageBuilder::new(&subscription_info) {
                        Ok(builder) => builder,
                        Err(e) => {
                                error!("Failed to create WebPushMessageBuilder: {:?}", e);
                                continue;
                        }
                };
                web_push_message_build.set_payload(ContentEncoding::Aes128Gcm, json_message_response.as_bytes());
                web_push_message_build.set_vapid_signature(signature);

                let client = match WebPushClient::new() {
                        Ok(client) => client,
                        Err(e) => {
                                error!("Failed to create WebPushClient: {:?}", e);
                                continue;
                        }
                };

                let web_push_message = match web_push_message_build.build() {
                        Ok(message) => message,
                        Err(e) => {
                                error!("Failed to build web push message: {:?}", e);
                                continue;
                        }
                };

                match client.send(web_push_message).await {
                        Ok(_) => info!("Web push notification sent successfully"),
                        Err(e) => error!("Failed to send web push message: {:?}", e),
                }
        }
}

pub async fn create_group_message(
        ctx: AppContext,
        group_id: String,
        message_request: MessageRequestDto,
) -> Result<MessageResponseDto, Error> {
        let group_id: i64 = group_id
                .parse()
                .map_err(|_| Error::new(ErrorCode::BadRequest, "Invalid group_id".into()))?;

        let auth_user = ctx.get_auth_user().await?;

        let group = ctx
                .group_repository
                .find_by_id_and_user_id(group_id, auth_user.id)?
                .ok_or(Error::new(ErrorCode::NotFound, "Group not found".into()))?;

        let message = {
                let mut id_generator = ctx.id_generator.lock().unwrap();
                ctx.message_repository.save(MessageWithSource {
                        id: id_generator.generate(),
                        created_at: Utc::now().naive_utc(),
                        updated_at: Utc::now().naive_utc(),
                        group_id: group.id,
                        source: auth_user.clone(),
                        content: message_request
                                .content
                                .clone()
                                .into_iter()
                                .filter_map(|(k, v)| k.parse::<i64>().ok().map(|key| (key, v)))
                                .collect(),
                        idempotency_key: message_request.idempotency_key.clone(),
                })?
        };

        let message_response = MessageResponseDto {
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
        };

        let message_response_for_notification = message_response.clone();

        tokio::spawn(async move {
                send_web_push_notifications(&ctx, group.clone(), message_response_for_notification).await;
        });

        Ok(message_response)
}
