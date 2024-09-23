use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use authorization::get_cached_token_data;
use axum::{routing::get, Json};
use controllers::{
        auth_controller, group_controller, message_controller, message_request_controller, user_controller,
        user_push_subscription_controller,
};
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use dtos::{
        MessageRequestDto, MessageRequestRequestDto, PresignedUploadUrlRequestDto, UserPushSubscriptionRequestDto,
        UserRequestDto,
};
use hyper::HeaderMap;
use models::User;
use repositories::{
        group_repository::GroupRepository, message_repository::MessageRepository,
        message_request_repository::MessageRequestRepository,
        user_push_subscription_repository::UserPushSubscriptionRepository, user_repository::UserRepository,
};
use rspc::{Config, Error, ErrorCode};
use serde_json::json;
use services::cognito_service::CognitoService;
use services::s3_service::S3Service;
use snowflake::SnowflakeIdGenerator;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

mod authorization;
mod controllers;
pub mod dtos;
pub mod models;
pub mod repositories;
pub mod schema;
pub mod services;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone)]
struct AppContext {
        auth_user_cache: Arc<RwLock<HashMap<String, User>>>,

        pub id_generator: Arc<Mutex<SnowflakeIdGenerator>>,
        pub headers: HeaderMap,
        pub sub: Arc<Mutex<Option<String>>>,

        pub cognito_service: CognitoService,
        pub s3_service: S3Service,

        pub group_repository: GroupRepository,
        pub message_repository: MessageRepository,
        pub message_request_repository: MessageRequestRepository,
        pub user_push_subscription_repository: UserPushSubscriptionRepository,
        pub user_repository: UserRepository,
}

impl AppContext {
        pub async fn get_auth_user(&self) -> Result<User, Error> {
                let sub =
                        self.sub.lock()
                                .map_err(|_| {
                                        tracing::error!("failed to retrieve sub from app data");
                                        Error::new(
                                                ErrorCode::InternalServerError,
                                                "Failed to retrieve sub from app data".into(),
                                        )
                                })?
                                .clone()
                                .ok_or_else(|| {
                                        tracing::error!("failed to retrieve sub from app data");
                                        Error::new(
                                                ErrorCode::InternalServerError,
                                                "Failed to retrieve sub from app data".into(),
                                        )
                                })?;

                let cache = self.auth_user_cache.read().await;
                if let Some(auth_user) = cache.get(&sub) {
                        return Ok(auth_user.clone());
                }
                drop(cache);

                let auth_user = self
                        .user_repository
                        .find_by_sub(sub.clone())?
                        .ok_or(Error::new(ErrorCode::NotFound, "Auth user not found".into()))?;

                let mut cache = self.auth_user_cache.write().await;
                cache.insert(sub.clone(), auth_user.clone());

                Ok(auth_user)
        }
}

#[tokio::main]
async fn main() {
        dotenv().ok();

        tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_ansi(false)
                .without_time()
                .init();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = r2d2::Pool::builder().build(manager).expect("failed to create pool.");
        pool.get()
                .expect("failed to get connection for migrations")
                .run_pending_migrations(MIGRATIONS)
                .expect("failed to run migrations");

        let auth_router = rspc::Router::<AppContext>::new().query("getAuthUser", |t| {
                t(|ctx: AppContext, _: ()| auth_controller::get_auth_user(ctx))
        });

        let group_router = rspc::Router::<AppContext>::new()
                .query("getGroup", |t| {
                        t(|ctx: AppContext, group_id: String| group_controller::get_group(ctx, group_id))
                })
                .query("getGroupMessages", |t| {
                        t(|ctx: AppContext, group_id: String| group_controller::get_group_messages(ctx, group_id))
                })
                .mutation("createGroupMessage", |t| {
                        t(
                                |ctx: AppContext, (group_id, message_request): (String, MessageRequestDto)| {
                                        group_controller::create_group_message(ctx, group_id, message_request)
                                },
                        )
                });

        let message_router = rspc::Router::<AppContext>::new().query("getMessages", |t| {
                t(|ctx: AppContext, _: ()| message_controller::get_messages(ctx))
        });

        let message_request_router = rspc::Router::<AppContext>::new()
                .query("getMessageRequest", |t| {
                        t(|ctx: AppContext, message_request_id: String| {
                                message_request_controller::get_message_request(ctx, message_request_id)
                        })
                })
                .mutation("createMessageRequest", |t| {
                        t(|ctx: AppContext, message_request_request: MessageRequestRequestDto| {
                                message_request_controller::create_message_request(ctx, message_request_request)
                        })
                })
                .mutation("approveMessageRequest", |t| {
                        t(|ctx: AppContext, message_request_id: String| {
                                message_request_controller::approve_message_request(ctx, message_request_id)
                        })
                });

        let users_router = rspc::Router::<AppContext>::new()
                .query("getUser", |t| {
                        t(|ctx: AppContext, user_id: String| user_controller::get_user(ctx, user_id))
                })
                .mutation("createUser", |t| {
                        t(|ctx: AppContext, user_request: UserRequestDto| {
                                user_controller::create_user(ctx, user_request)
                        })
                })
                .mutation("createUserProfilePicturePresignedUploadUrl", |t| {
                        t(
                                |ctx: AppContext, presigned_upload_url_request: PresignedUploadUrlRequestDto| {
                                        user_controller::create_user_profile_picture_presigned_upload_url(
                                                ctx,
                                                presigned_upload_url_request,
                                        )
                                },
                        )
                });

        let user_push_subscriptions_router =
                rspc::Router::<AppContext>::new().mutation("createUserPushSubscription", |t| {
                        t(
                                |ctx: AppContext, user_push_subscription_request: UserPushSubscriptionRequestDto| {
                                        user_push_subscription_controller::create_user_push_subscripition(
                                                ctx,
                                                user_push_subscription_request,
                                        )
                                },
                        )
                });

        let router = rspc::Router::<AppContext>::new()
                .config(Config::new().export_ts_bindings(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gen.ts")))
                .query("version", |t| t(|_, _: ()| "0.1.0"))
                .middleware(|mw| {
                        mw.middleware(|mw| async move {
                                let authorization = mw.ctx.headers.get("Authorization").ok_or(Error::new(
                                        ErrorCode::Unauthorized,
                                        "Missing Authorization header".into(),
                                ))?;

                                let bearer_token = authorization.to_str().map_err(|_| {
                                        Error::new(ErrorCode::Unauthorized, "Bearer token invalid".into())
                                })?;

                                if !bearer_token.starts_with("Bearer ") {
                                        return Err(Error::new(ErrorCode::Unauthorized, "Bearer token invalid".into()));
                                }

                                let token = &bearer_token[7..];

                                let token_data = get_cached_token_data(token)
                                        .await
                                        .map_err(|_| Error::new(ErrorCode::Unauthorized, "Token invalid".into()))?;

                                {
                                        let mut sub = mw.ctx.sub.lock().map_err(|_| {
                                                tracing::error!("failed to retrieve sub from app data");
                                                Error::new(
                                                        ErrorCode::InternalServerError,
                                                        "Failed to retrieve sub from app data".into(),
                                                )
                                        })?;
                                        *sub = Some(token_data.claims.sub);
                                }

                                Ok(mw)
                        })
                })
                .merge("auth.", auth_router)
                .merge("groups.", group_router)
                .merge("messages.", message_router)
                .merge("messageRequests.", message_request_router)
                .merge("users.", users_router)
                .merge("userPushSubscriptions.", user_push_subscriptions_router)
                .build()
                .arced();

        let aws_config = aws_config::load_from_env().await;

        let app = axum::Router::new()
                .route("/health", get(|| async { Json(json!({ "status": "up" })) }))
                .nest(
                        "/rspc",
                        rspc_axum::endpoint(router, move |headers: HeaderMap| AppContext {
                                auth_user_cache: Arc::new(RwLock::new(HashMap::new())),

                                id_generator: Arc::new(Mutex::new(SnowflakeIdGenerator::new(1, 1))),
                                headers,
                                sub: Arc::new(Mutex::new(None)),

                                cognito_service: CognitoService::new(aws_sdk_cognitoidentityprovider::Client::new(
                                        &aws_config,
                                )),
                                s3_service: S3Service::new(aws_sdk_s3::Client::new(&aws_config)),

                                group_repository: GroupRepository::new(pool.clone()),
                                message_repository: MessageRepository::new(pool.clone()),
                                message_request_repository: MessageRequestRepository::new(pool.clone()),
                                user_push_subscription_repository: UserPushSubscriptionRepository::new(pool.clone()),
                                user_repository: UserRepository::new(pool.clone()),
                        }),
                )
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive());

        let host = env::var("SERVER_HOST").unwrap_or("0.0.0.0".to_string());
        let port = env::var("SERVER_PORT").unwrap_or("3000".to_string());
        let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
                .await
                .unwrap();
        axum::serve(listener, app).await.unwrap();
}
