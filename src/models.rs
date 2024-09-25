use crate::schema;
use diesel::prelude::*;
use std::collections::HashMap;

#[derive(Queryable, Identifiable, Selectable, Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub sub: String,
        pub email: String,
        pub first_name: String,
        pub last_name: String,
        pub display_name: Option<String>,
        pub public_key: String,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = schema::message_requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MessageRequest {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub source_id: i64,
        pub destination_id: i64,
        pub approved_at: Option<chrono::NaiveDateTime>,
}

pub struct MessageRequestWithRelationships {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub source: User,
        pub destination: User,
        pub approved_at: Option<chrono::NaiveDateTime>,
}

impl From<(MessageRequest, User, User)> for MessageRequestWithRelationships {
        fn from((message_request, source, destination): (MessageRequest, User, User)) -> Self {
                MessageRequestWithRelationships {
                        id: message_request.id,
                        created_at: message_request.created_at,
                        updated_at: message_request.updated_at,
                        source,
                        destination,
                        approved_at: message_request.approved_at,
                }
        }
}

#[derive(Queryable, Identifiable, Selectable, Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = schema::groups)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Group {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub name: Option<String>,
        pub message_request_id: Option<i64>,
}

#[derive(Queryable, Identifiable, Selectable, Insertable, Associations, AsChangeset, Debug, Clone)]
#[diesel(belongs_to(Group))]
#[diesel(table_name = schema::group_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GroupUser {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub group_id: i64,
        pub user_id: i64,
        pub is_admin: bool,
        pub nickname: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GroupWithRelationships {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub name: Option<String>,
        pub message_request_id: Option<i64>,
        pub users: Vec<GroupUserWithRelationships>,
}

#[derive(Debug, Clone)]
pub struct GroupUserWithRelationships {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub user: User,
        pub is_admin: bool,
        pub nickname: Option<String>,
}

impl From<(Group, Vec<GroupUserWithRelationships>)> for GroupWithRelationships {
        fn from((group, users): (Group, Vec<GroupUserWithRelationships>)) -> Self {
                GroupWithRelationships {
                        id: group.id,
                        created_at: group.created_at,
                        updated_at: group.updated_at,
                        name: group.name,
                        message_request_id: group.message_request_id,
                        users,
                }
        }
}

impl From<(GroupUser, User)> for GroupUserWithRelationships {
        fn from((group_user, user): (GroupUser, User)) -> Self {
                GroupUserWithRelationships {
                        id: group_user.id,
                        created_at: group_user.created_at,
                        updated_at: group_user.updated_at,
                        user,
                        is_admin: group_user.is_admin,
                        nickname: group_user.nickname,
                }
        }
}

#[derive(Queryable, Identifiable, Selectable, Insertable, Associations, AsChangeset, Debug, PartialEq)]
#[diesel(belongs_to(Group))]
#[diesel(table_name = schema::messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Message {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub group_id: i64,
        pub source_id: i64,
        pub idempotency_key: Option<String>,
}

#[derive(Queryable, Identifiable, Selectable, Insertable, Associations, AsChangeset, Debug, Clone)]
#[diesel(belongs_to(Message))]
#[diesel(primary_key(message_id, user_id))]
#[diesel(table_name = schema::message_content)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MessageContent {
        pub message_id: i64,
        pub user_id: i64,
        pub content: String,
}

#[derive(Debug, Clone)]
pub struct MessageWithSource {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub group_id: i64,
        pub source: User,
        pub content: HashMap<i64, String>,
        pub idempotency_key: Option<String>,
}

impl From<(Message, User, HashMap<i64, String>)> for MessageWithSource {
        fn from((message, source, content): (Message, User, HashMap<i64, String>)) -> Self {
                MessageWithSource {
                        id: message.id,
                        created_at: message.created_at,
                        updated_at: message.updated_at,
                        group_id: message.group_id,
                        source,
                        content,
                        idempotency_key: message.idempotency_key,
                }
        }
}

#[derive(Debug, Clone)]
pub struct MessageWithGroup {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub group: GroupWithRelationships,
        pub source: User,
        pub content: String,
        pub idempotency_key: Option<String>,
}

impl From<(Message, GroupWithRelationships, User, MessageContent)> for MessageWithGroup {
        fn from((message, group, source, content): (Message, GroupWithRelationships, User, MessageContent)) -> Self {
                MessageWithGroup {
                        id: message.id,
                        created_at: message.created_at,
                        updated_at: message.updated_at,
                        group,
                        source,
                        content: content.content,
                        idempotency_key: message.idempotency_key,
                }
        }
}

#[derive(Debug, Clone)]
pub struct MessageWithRelationships {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub group: GroupWithRelationships,
        pub source: User,
        pub content: HashMap<i64, String>,
        pub idempotency_key: Option<String>,
}

impl From<(Message, GroupWithRelationships, User, HashMap<i64, String>)> for MessageWithRelationships {
        fn from(
                (message, group, source, content): (Message, GroupWithRelationships, User, HashMap<i64, String>),
        ) -> Self {
                MessageWithRelationships {
                        id: message.id,
                        created_at: message.created_at,
                        updated_at: message.updated_at,
                        group,
                        source,
                        content,
                        idempotency_key: message.idempotency_key,
                }
        }
}

#[derive(Queryable, Identifiable, Selectable, Insertable, Associations, AsChangeset, Debug, Clone)]
#[diesel(belongs_to(User))]
#[diesel(table_name = schema::user_push_subscriptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPushSubscription {
        pub id: i64,
        pub created_at: chrono::NaiveDateTime,
        pub updated_at: chrono::NaiveDateTime,
        pub user_id: i64,
        pub endpoint: String,
        pub p256dh: String,
        pub auth: String,
}
