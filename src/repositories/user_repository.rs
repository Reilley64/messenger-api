use crate::errors::problem::Problem;
use crate::models::User;
use crate::schema;
use crate::schema::users;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

#[derive(Debug, Clone)]
pub struct UserRepository {
        pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl UserRepository {
        pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
                Self { pool }
        }

        pub fn find_by_id(&self, user_id: i64) -> Result<Option<User>, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                users::table
                        .find(user_id)
                        .first(&mut connection)
                        .optional()
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))
        }

        pub fn find_by_sub(&self, sub: String) -> Result<Option<User>, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                users::table
                        .filter(users::sub.eq(sub))
                        .first(&mut connection)
                        .optional()
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))
        }

        pub fn exists_by_sub(&self, sub: String) -> Result<bool, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                diesel::select(diesel::dsl::exists(users::table.filter(schema::users::sub.eq(sub))))
                        .get_result(&mut connection)
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))
        }

        pub fn save(&self, user: User) -> Result<User, Problem> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Problem::InternalServerError("failed to pool connection".to_string()))?;

                diesel::insert_into(users::table)
                        .values(&user)
                        .on_conflict(users::id)
                        .do_update()
                        .set(&user)
                        .get_result(&mut connection)
                        .optional()
                        .map_err(|_| Problem::InternalServerError("failed to query database".to_string()))?
                        .ok_or(Problem::InternalServerError("failed to query database".to_string()))
        }
}
