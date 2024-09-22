use derive_new::new;
use diesel::prelude::*;
use rspc::{Error, ErrorCode};

use crate::models::User;
use crate::schema::users;
use crate::DbPool;

#[derive(new, Debug, Clone)]
pub struct UserRepository {
        pool: DbPool,
}

impl UserRepository {
        pub fn find_by_id(&self, user_id: i64) -> Result<Option<User>, Error> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                users::table
                        .find(user_id)
                        .first(&mut connection)
                        .optional()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))
        }

        pub fn find_by_sub(&self, sub: String) -> Result<Option<User>, Error> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                users::table
                        .filter(users::sub.eq(sub))
                        .first(&mut connection)
                        .optional()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))
        }

        pub fn exists_by_sub(&self, sub: String) -> Result<bool, Error> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                diesel::select(diesel::dsl::exists(users::table.filter(users::sub.eq(sub))))
                        .get_result(&mut connection)
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))
        }

        pub fn save(&self, user: User) -> Result<User, Error> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                diesel::insert_into(users::table)
                        .values(&user)
                        .on_conflict(users::id)
                        .do_update()
                        .set(&user)
                        .get_result(&mut connection)
                        .optional()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))?
                        .ok_or(Error::new(
                                ErrorCode::InternalServerError,
                                "Failed to query database".into(),
                        ))
        }
}
