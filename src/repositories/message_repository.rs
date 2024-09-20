use std::collections::HashMap;

use crate::errors::problem::Problem;
use crate::models::{
        Group, GroupUser, GroupUserWithRelationships, GroupWithRelationships, Message, MessageContent,
        MessageWithGroup, MessageWithSource, User,
};
use crate::schema::{group_users, groups, message_content, messages, users};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

#[derive(Debug, Clone)]
pub struct MessageRepository {
        pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl MessageRepository {
        pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
                Self { pool }
        }

        pub fn find_by_group_id(&self, group_id: i64) -> Result<Vec<MessageWithSource>, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                let messages: Vec<(Message, User)> = messages::table
                        .inner_join(users::table.on(users::id.eq(messages::source_id)))
                        .filter(messages::group_id.eq(group_id))
                        .select((messages::all_columns, users::all_columns))
                        .order_by(messages::id.desc())
                        .load::<(Message, User)>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?;

                let message_with_relationships =
                        messages.into_iter()
                                .map(|(message, user)| {
                                        let content = MessageContent::belonging_to(&message)
                                                .load(&mut connection)
                                                .map_err(|_| {
                                                        Problem::InternalServerError(
                                                                "failed to query database".to_string(),
                                                        )
                                                })?;
                                        let content_map: HashMap<i64, String> = content
                                                .into_iter()
                                                .map(|mc: MessageContent| (mc.user_id, mc.content))
                                                .collect();
                                        Ok(MessageWithSource::from((message, user, content_map)))
                                })
                                .collect::<Result<Vec<MessageWithSource>, Problem>>()?;

                Ok(message_with_relationships)
        }

        pub fn find_by_user_id(&self, user_id: i64) -> Result<Vec<MessageWithGroup>, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                let messages = message_content::table
                        .inner_join(messages::table.on(messages::id.eq(message_content::message_id)))
                        .inner_join(users::table.on(users::id.eq(messages::source_id)))
                        .filter(message_content::user_id.eq(user_id))
                        .distinct_on(messages::group_id)
                        .order_by((messages::group_id, message_content::message_id.desc()))
                        .select((messages::all_columns, users::all_columns, message_content::all_columns))
                        .load::<(Message, User, MessageContent)>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?;

                let messages_with_group = messages
                        .into_iter()
                        .map(|(message, source, content)| {
                                let group = groups::table
                                        .find(message.group_id)
                                        .first::<Group>(&mut connection)
                                        .map_err(|_| {
                                                Problem::InternalServerError("failed to query database".to_string())
                                        })?;

                                let group_users = group_users::table
                                        .filter(group_users::group_id.eq(group.id))
                                        .select(group_users::all_columns)
                                        .load::<GroupUser>(&mut connection)
                                        .map_err(|_| {
                                                Problem::InternalServerError("failed to query database".to_string())
                                        })?;

                                let group_users_with_relationships: Vec<GroupUserWithRelationships> = group_users
                                        .into_iter()
                                        .map(|gu| {
                                                let user = users::table
                                                        .find(gu.user_id)
                                                        .first::<User>(&mut connection)
                                                        .optional()
                                                        .map_err(|_| {
                                                                Problem::InternalServerError(
                                                                        "failed to query database".to_string(),
                                                                )
                                                        })?
                                                        .ok_or(Problem::InternalServerError(
                                                                "failed to query database".to_string(),
                                                        ))?;
                                                Ok::<GroupUserWithRelationships, Problem>(
                                                        GroupUserWithRelationships::from((gu.clone(), user)),
                                                )
                                        })
                                        .collect::<Result<Vec<GroupUserWithRelationships>, Problem>>()?;

                                let group_with_relationships =
                                        GroupWithRelationships::from((group, group_users_with_relationships));

                                Ok(MessageWithGroup::from((
                                        message,
                                        group_with_relationships,
                                        source,
                                        content,
                                )))
                        })
                        .collect::<Result<Vec<MessageWithGroup>, Problem>>()?;

                Ok(messages_with_group)
        }

        pub fn save(&self, messages_with_source: MessageWithSource) -> Result<MessageWithSource, Problem> {
                let message = Message {
                        id: messages_with_source.id,
                        created_at: messages_with_source.created_at,
                        updated_at: messages_with_source.updated_at,
                        group_id: messages_with_source.group_id,
                        source_id: messages_with_source.source.id,
                        idempotency_key: messages_with_source.idempotency_key,
                };

                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                let message = diesel::insert_into(messages::table)
                        .values(&message)
                        .on_conflict(messages::id)
                        .do_update()
                        .set(&message)
                        .get_result::<Message>(&mut connection)
                        .optional()
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?
                        .ok_or(Problem::InternalServerError("failed to query database".to_string()))?;

                for (user_id, content) in &messages_with_source.content {
                        let message_content = MessageContent {
                                message_id: message.id,
                                user_id: user_id.clone(),
                                content: content.clone(),
                        };

                        diesel::insert_into(message_content::table)
                                .values(&message_content)
                                .on_conflict((message_content::message_id, message_content::user_id))
                                .do_update()
                                .set(&message_content)
                                .execute(&mut connection)
                                .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?;
                }

                let source = users::table
                        .find(message.source_id)
                        .first(&mut connection)
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?;

                Ok(MessageWithSource::from((message, source, messages_with_source.content)))
        }
}
