use diesel::alias;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

use crate::errors::problem::Problem;
use crate::models::{MessageRequest, MessageRequestWithRelationships, User};
use crate::schema::message_requests;
use crate::schema::users;

#[derive(Debug, Clone)]
pub struct MessageRequestRepository {
        pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl MessageRequestRepository {
        pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
                Self { pool }
        }

        pub fn find_by_id_and_destination_id(
                &self,
                message_request_id: i64,
                destination_id: i64,
        ) -> Result<Option<MessageRequestWithRelationships>, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                let destination_users = alias!(users as destination_users);

                let response = message_requests::table
                        .inner_join(users::table.on(users::id.eq(message_requests::source_id)))
                        .inner_join(
                                destination_users
                                        .on(destination_users.field(users::id).eq(message_requests::destination_id)),
                        )
                        .filter(message_requests::id
                                .eq(message_request_id)
                                .and(message_requests::destination_id.eq(destination_id)))
                        .select((message_requests::all_columns, users::all_columns, users::all_columns))
                        .first::<(MessageRequest, User, User)>(&mut connection)
                        .optional()
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?;

                response.map_or(Ok(None), |(message_request, source, destination)| {
                        Ok(Some(MessageRequestWithRelationships::from((
                                message_request,
                                source,
                                destination,
                        ))))
                })
        }

        pub fn exists_by_source_id_and_destination_id(
                &self,
                source_id: i64,
                destination_id: i64,
        ) -> Result<bool, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                diesel::select(diesel::dsl::exists(
                        message_requests::table.filter(message_requests::source_id
                                .eq(source_id)
                                .and(message_requests::destination_id.eq(destination_id))),
                ))
                .get_result(&mut connection)
                .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))
        }

        pub fn save(
                &self,
                message_request_with_relationships: MessageRequestWithRelationships,
        ) -> Result<MessageRequestWithRelationships, Problem> {
                let message_request = MessageRequest {
                        id: message_request_with_relationships.id,
                        created_at: message_request_with_relationships.created_at,
                        updated_at: message_request_with_relationships.updated_at,
                        source_id: message_request_with_relationships.source.id,
                        destination_id: message_request_with_relationships.destination.id,
                        approved_at: message_request_with_relationships.approved_at,
                };

                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                let message_request = diesel::insert_into(message_requests::table)
                        .values(&message_request)
                        .on_conflict(message_requests::id)
                        .do_update()
                        .set(&message_request)
                        .get_result::<MessageRequest>(&mut connection)
                        .optional()
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?
                        .ok_or(Problem::InternalServerError("failed to query database".to_string()))?;

                let source = users::table
                        .find(message_request.source_id)
                        .first(&mut connection)
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?;
                let destination = users::table
                        .find(message_request.destination_id)
                        .first(&mut connection)
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?;

                Ok(MessageRequestWithRelationships::from((
                        message_request,
                        source,
                        destination,
                )))
        }
}
