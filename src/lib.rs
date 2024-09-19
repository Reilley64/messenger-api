use actix_web::dev::Payload;
use actix_web::http::Error;
use actix_web::{FromRequest, HttpRequest};
use apistos::ApiSecurity;
use futures_util::future::{ready, Ready};
use repositories::group_repository::GroupRepository;
use repositories::message_repository::MessageRepository;
use snowflake::SnowflakeIdGenerator;
use std::sync::Mutex;

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
