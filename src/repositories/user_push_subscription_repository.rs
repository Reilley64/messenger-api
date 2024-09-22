use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use rspc::{Error, ErrorCode};

use crate::{models::UserPushSubscription, schema::user_push_subscriptions};

#[derive(Debug, Clone)]
pub struct UserPushSubscriptionRepository {
        pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl UserPushSubscriptionRepository {
        pub fn new(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
                Self { pool }
        }

        pub fn find_by_user_id_order_by_created_at_desc(
                &self,
                user_id: i64,
        ) -> Result<Option<UserPushSubscription>, Error> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                let user_push_subscription = user_push_subscriptions::table
                        .filter(user_push_subscriptions::user_id.eq(user_id))
                        .order_by(user_push_subscriptions::created_at.desc())
                        .first::<UserPushSubscription>(&mut connection)
                        .optional()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))?;

                Ok(user_push_subscription)
        }

        pub fn save(&self, user_push_subscription: UserPushSubscription) -> Result<UserPushSubscription, Error> {
                let mut connection = self
                        .pool
                        .get()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to pool connection".into()))?;

                diesel::insert_into(user_push_subscriptions::table)
                        .values(&user_push_subscription)
                        .on_conflict(user_push_subscriptions::id)
                        .do_update()
                        .set(&user_push_subscription)
                        .get_result(&mut connection)
                        .optional()
                        .map_err(|_| Error::new(ErrorCode::InternalServerError, "Failed to query database".into()))?
                        .ok_or(Error::new(
                                ErrorCode::InternalServerError,
                                "Failed to query database".into(),
                        ))
        }
}
