use std::collections::HashMap;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

use crate::errors::problem::Problem;
use crate::models::{
        Group, GroupUser, GroupUserWithRelationships, GroupWithRelationships, Message, MessageContent,
        MessageWithGroup, MessageWithSource, User,
};
use crate::schema::{groups, message_content, messages, users};

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
                        .map_err(|_| Problem::InternalServerError("Failed to pool connection".to_string()))?;

                let messages = messages::table
                        .filter(messages::group_id.eq(group_id))
                        .order_by(messages::id.desc())
                        .load::<Message>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("Failed to query database".to_string()))?;

                let users = users::table
                        .filter(users::id.eq_any(messages.iter().map(|m| m.source_id)))
                        .load::<User>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("Failed to query database".to_string()))?;
                let user_map: HashMap<i64, User> = users.into_iter().map(|u| (u.id, u)).collect();

                let content = MessageContent::belonging_to(&messages)
                        .load::<MessageContent>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("Failed to query database".to_string()))?;
                let content_map: HashMap<i64, Vec<MessageContent>> =
                        content.into_iter().fold(HashMap::new(), |mut acc, content_item| {
                                acc.entry(content_item.message_id)
                                        .or_insert_with(Vec::new)
                                        .push(content_item);
                                acc
                        });

                let message_with_relationships = messages
                        .into_iter()
                        .map(|m| {
                                let user = user_map
                                        .get(&m.source_id)
                                        .ok_or(Problem::InternalServerError("failed to query database".to_string()))?;
                                let content = content_map.get(&m.id).cloned().unwrap_or_default();
                                let content_map: HashMap<i64, String> =
                                        content.into_iter().map(|mc| (mc.user_id, mc.content.clone())).collect();
                                Ok(MessageWithSource::from((m, user.clone(), content_map)))
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

                let groups = groups::table
                        .filter(groups::id.eq_any(&messages.iter().map(|(m, _, _)| m.group_id).collect::<Vec<i64>>()))
                        .load::<Group>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("Failed to query database".to_string()))?;
                let group_map: HashMap<i64, Group> = groups.iter().map(|g| (g.id, g.clone())).collect();

                let group_users = GroupUser::belonging_to(&groups)
                        .load::<GroupUser>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("Failed to query database".to_string()))?;

                let users = users::table
                        .filter(users::id.eq_any(&group_users.iter().map(|gu| gu.user_id).collect::<Vec<i64>>()))
                        .load::<User>(&mut connection)
                        .map_err(|_| Problem::InternalServerError("Failed to query database".to_string()))?;
                let user_map: HashMap<i64, User> = users.iter().map(|u| (u.id, u.clone())).collect();

                let group_users_with_relationships_map: HashMap<i64, Vec<GroupUserWithRelationships>> = group_users
                        .into_iter()
                        .filter_map(|gu| {
                                user_map.get(&gu.user_id).map(|user| {
                                        (gu.group_id, GroupUserWithRelationships::from((gu, user.clone())))
                                })
                        })
                        .fold(HashMap::new(), |mut acc, (group_id, gu_with_rel)| {
                                acc.entry(group_id).or_insert_with(Vec::new).push(gu_with_rel);
                                acc
                        });

                let messages_with_group = messages
                        .into_iter()
                        .filter_map(|(message, source, content)| {
                                group_map.get(&message.group_id).map(|group| {
                                        let group_users = group_users_with_relationships_map
                                                .get(&group.id)
                                                .cloned()
                                                .unwrap_or_default();
                                        let group_with_relationships =
                                                GroupWithRelationships::from((group.clone(), group_users));
                                        MessageWithGroup::from((message, group_with_relationships, source, content))
                                })
                        })
                        .collect();

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
