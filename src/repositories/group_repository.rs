use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use rspc::{Error, ErrorCode};

use crate::{
        models::{Group, GroupUser, GroupUserWithRelationships, GroupWithRelationships, User},
        schema::{group_users, groups, users},
};

#[derive(Debug, Clone)]
pub struct GroupRepository {
        pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl GroupRepository {
        pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
                Self { pool }
        }

        pub fn find_by_id_and_user_id(
                &self,
                group_id: i64,
                user_id: i64,
        ) -> Result<Option<GroupWithRelationships>, Error> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                let group = groups::table
                        .find(group_id)
                        .first::<Group>(&mut connection)
                        .optional()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))?;

                group.map_or(Ok(None), |group| {
                        let group_users = group_users::table
                                .filter(group_users::group_id.eq(group.id))
                                .select(group_users::all_columns)
                                .load::<GroupUser>(&mut connection)
                                .map_err(|_| {
                                        Error::new(ErrorCode::InternalServerError, "Failed to query database".into())
                                })?;

                        if !group_users.iter().any(|gu: &GroupUser| gu.user_id == user_id) {
                                return Ok(None);
                        }

                        let group_users_with_relationships: Vec<GroupUserWithRelationships> = group_users
                                .into_iter()
                                .map(|gu| {
                                        let user = users::table
                                                .find(gu.user_id)
                                                .first::<User>(&mut connection)
                                                .optional()
                                                .map_err(|_| {
                                                        Error::new(
                                                                ErrorCode::InternalServerError,
                                                                "failed to query database".into(),
                                                        )
                                                })?
                                                .ok_or(Error::new(
                                                        ErrorCode::InternalServerError,
                                                        "failed to query database".into(),
                                                ))?;
                                        Ok(GroupUserWithRelationships::from((gu.clone(), user)))
                                })
                                .collect::<Result<Vec<GroupUserWithRelationships>, Error>>()?;

                        Ok(Some(GroupWithRelationships::from((
                                group,
                                group_users_with_relationships,
                        ))))
                })
        }

        pub fn save(&self, group_with_relationships: GroupWithRelationships) -> Result<Group, Error> {
                let group = Group {
                        id: group_with_relationships.id,
                        created_at: group_with_relationships.created_at,
                        updated_at: group_with_relationships.updated_at,
                        name: group_with_relationships.name,
                        message_request_id: group_with_relationships.message_request_id,
                };

                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                let group = diesel::insert_into(groups::table)
                        .values(&group)
                        .on_conflict(groups::id)
                        .do_update()
                        .set(&group)
                        .get_result::<Group>(&mut connection)
                        .optional()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))?
                        .ok_or(Error::new(
                                ErrorCode::InternalServerError,
                                "Failed to query database".into(),
                        ))?;

                for gu in group_with_relationships.users.into_iter() {
                        let group_user = GroupUser {
                                id: gu.id,
                                created_at: gu.created_at,
                                updated_at: gu.updated_at,
                                group_id: group.id,
                                user_id: gu.user.id,
                                is_admin: gu.is_admin,
                                nickname: gu.nickname,
                        };

                        diesel::insert_into(group_users::table)
                                .values(&group_user)
                                .on_conflict(group_users::id)
                                .do_update()
                                .set(&group_user)
                                .execute(&mut connection)
                                .map_err(|_| {
                                        Error::new(ErrorCode::InternalServerError, "Failed to query database".into())
                                })?;
                }

                Ok(group)
        }
}
