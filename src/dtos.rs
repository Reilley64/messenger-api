use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{GroupWithRelationships, MessageRequestWithRelationships, User};

#[derive(Clone, Deserialize, JsonSchema, ApiComponent, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserRequestDto {
        pub public_key: String,
}

#[derive(Serialize, JsonSchema, ApiComponent, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserResponseDto {
        pub id: String,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub name: String,
        pub public_key: String,
}

impl From<User> for UserResponseDto {
        fn from(user: User) -> Self {
                UserResponseDto {
                        id: user.id.to_string(),
                        created_at: user.created_at,
                        updated_at: user.updated_at,
                        name: user
                                .display_name
                                .unwrap_or(vec![user.first_name, user.last_name].join(" ")),
                        public_key: user.public_key,
                }
        }
}

#[derive(Clone, Deserialize, JsonSchema, ApiComponent, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessageRequestRequestDto {
        pub destination_id: String,
}

#[derive(Serialize, JsonSchema, ApiComponent, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessageRequestResponseDto {
        pub id: String,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub source: UserResponseDto,
        pub destination: UserResponseDto,
        pub approved_at: Option<chrono::NaiveDateTime>,
}

impl From<MessageRequestWithRelationships> for MessageRequestResponseDto {
        fn from(message_request: MessageRequestWithRelationships) -> Self {
                MessageRequestResponseDto {
                        id: message_request.id.to_string(),
                        created_at: message_request.created_at,
                        updated_at: message_request.updated_at,
                        source: UserResponseDto::from(message_request.source),
                        destination: UserResponseDto::from(message_request.destination),
                        approved_at: message_request.approved_at,
                }
        }
}

#[derive(Serialize, JsonSchema, ApiComponent)]
#[serde(rename_all = "camelCase")]
pub struct GroupResponseDto {
        pub id: String,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub name: String,
        pub message_request_id: Option<i64>,
        pub users: Vec<UserResponseDto>,
}

impl From<GroupWithRelationships> for GroupResponseDto {
        fn from(group: GroupWithRelationships) -> Self {
                GroupResponseDto {
                        id: group.id.to_string(),
                        created_at: group.created_at,
                        updated_at: group.updated_at,
                        name: group.name.unwrap_or_else(|| {
                                group.users
                                        .iter()
                                        .map(|gu| format!("{} {}", gu.user.first_name, gu.user.last_name))
                                        .collect::<Vec<String>>()
                                        .join(", ")
                        }),
                        message_request_id: group.message_request_id,
                        users: group
                                .users
                                .iter()
                                .map(|gu| {
                                        let mut user = UserResponseDto::from(gu.user.clone());
                                        user.name = gu
                                                .nickname
                                                .as_ref()
                                                .map_or(user.name.clone(), |nickname| nickname.clone());
                                        user
                                })
                                .collect(),
                }
        }
}

#[derive(Deserialize, JsonSchema, ApiComponent, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageRequestDto {
        pub content: HashMap<String, String>,
        pub idempotency_key: Option<String>,
}

#[derive(Serialize, JsonSchema, ApiComponent)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponseDto {
        pub id: String,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub source: UserResponseDto,
        pub content: String,
        pub idempotency_key: Option<String>,
}

#[derive(Serialize, JsonSchema, ApiComponent)]
#[serde(rename_all = "camelCase")]
pub struct MessageWithGroupResponseDto {
        pub id: String,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub group: GroupResponseDto,
        pub source: UserResponseDto,
        pub content: String,
        pub idempotency_key: Option<String>,
}
