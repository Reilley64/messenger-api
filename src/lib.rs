use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_web::dev::Payload;
use actix_web::http::Error;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest};
use apistos::ApiSecurity;
use errors::problem::Problem;
use futures_util::future::{ready, Ready};
use log::error;
use models::User;
use repositories::group_repository::GroupRepository;
use repositories::message_repository::MessageRepository;
use snowflake::SnowflakeIdGenerator;
use tokio::sync::RwLock;

use crate::repositories::message_request_repository::MessageRequestRepository;
use crate::repositories::user_repository::UserRepository;

pub mod controllers;
pub mod dtos;
pub mod errors;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod schema;

#[derive(Debug)]
pub struct AppState {
        pub sub: Mutex<Option<String>>,

        pub id_generator: Mutex<SnowflakeIdGenerator>,

        pub group_repository: GroupRepository,
        pub message_repositoy: MessageRepository,
        pub message_request_repository: MessageRequestRepository,
        pub user_repository: UserRepository,

        pub auth_user_cache: Arc<RwLock<HashMap<String, User>>>,
}

#[derive(ApiSecurity)]
#[openapi_security(scheme(security_type(http(scheme = "bearer"))))]
pub struct BearerAuth;

impl FromRequest for BearerAuth {
        type Error = Error;
        type Future = Ready<Result<Self, Self::Error>>;

        fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
                ready(Ok(BearerAuth {}))
        }
}

pub async fn get_auth_user_from_cache(data: &Data<AppState>) -> Result<User, Problem> {
        let sub =
                data.sub.lock()
                        .map_err(|_| {
                                error!("failed to retrieve sub from app data");
                                Problem::InternalServerError("Failed to retrieve sub from app data".to_string())
                        })?
                        .clone()
                        .ok_or_else(|| {
                                error!("failed to retrieve sub from app data");
                                Problem::InternalServerError("Failed to retrieve sub from sapp data".to_string())
                        })?;

        let cache = data.auth_user_cache.read().await;
        if let Some(auth_user) = cache.get(&sub) {
                return Ok(auth_user.clone());
        }
        drop(cache);

        let auth_user = data
                .user_repository
                .find_by_sub(sub.clone())?
                .ok_or(Problem::InternalServerError("Auth user not found".to_string()))?;

        let mut cache = data.auth_user_cache.write().await;
        cache.insert(sub, auth_user.clone());

        Ok(auth_user)
}
