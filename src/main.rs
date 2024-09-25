use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use authorization::get_cached_token_data;
use axum::http::request::Parts;
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
        MessageRequestDto, MessageRequestRequestDto, MessageWithGroupResponseDto, PresignedUploadUrlRequestDto,
        UserPushSubscriptionRequestDto, UserRequestDto,
};
use models::User;
use repositories::{
        group_repository::GroupRepository, message_repository::MessageRepository,
        message_request_repository::MessageRequestRepository,
        user_push_subscription_repository::UserPushSubscriptionRepository, user_repository::UserRepository,
};
use rspc::{Config, Error, ErrorCode};
use serde_json::json;
use services::google_cloud_storage_service::GoogleCloudStorageService;
use snowflake::SnowflakeIdGenerator;
use tokio::sync::{
        broadcast::{self, Sender},
        RwLock,
};
use tokio_stream::wrappers::BroadcastStream;
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

struct RequestContext {
        parts: Parts,
        sub: Option<String>,
        app_state: Arc<AppState>,
}

struct AppState {
        auth_user_cache: Arc<RwLock<HashMap<String, User>>>,
        message_senders: Arc<RwLock<HashMap<i64, Sender<MessageWithGroupResponseDto>>>>,
        id_generator: Arc<Mutex<SnowflakeIdGenerator>>,

        google_cloud_storage_service: GoogleCloudStorageService,

        group_repository: GroupRepository,
        message_repository: MessageRepository,
        message_request_repository: MessageRequestRepository,
        user_push_subscription_repository: UserPushSubscriptionRepository,
        user_repository: UserRepository,
}

impl RequestContext {
        pub async fn get_auth_user(&self) -> Result<User, Error> {
                let sub = self.sub.as_ref().ok_or_else(|| {
                        tracing::error!("failed to retrieve sub from request context");
                        Error::new(
                                ErrorCode::InternalServerError,
                                "Failed to retrieve sub from request context".into(),
                        )
                })?;

                let cache = self.app_state.auth_user_cache.read().await;
                if let Some(auth_user) = cache.get(sub) {
                        return Ok(auth_user.clone());
                }
                drop(cache);

                let auth_user = self
                        .app_state
                        .user_repository
                        .find_by_sub(sub.clone())?
                        .ok_or(Error::new(ErrorCode::NotFound, "Auth user not found".into()))?;

                let mut cache = self.app_state.auth_user_cache.write().await;
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

        let auth_router = rspc::Router::<RequestContext>::new().query("getAuthUser", |t| {
                t(|ctx: RequestContext, _: ()| auth_controller::get_auth_user(ctx))
        });

        let group_router = rspc::Router::<RequestContext>::new()
                .query("getGroup", |t| {
                        t(|ctx: RequestContext, group_id: String| group_controller::get_group(ctx, group_id))
                })
                .query("getGroupMessages", |t| {
                        t(
                                |ctx: RequestContext, group_id: String| {
                                        group_controller::get_group_messages(ctx, group_id)
                                },
                        )
                })
                .mutation("createGroupMessage", |t| {
                        t(
                                |ctx: RequestContext, (group_id, message_request): (String, MessageRequestDto)| {
                                        group_controller::create_group_message(ctx, group_id, message_request)
                                },
                        )
                });

        let message_router = rspc::Router::<RequestContext>::new()
                .query("getMessages", |t| {
                        t(|ctx: RequestContext, _: ()| message_controller::get_messages(ctx))
                })
                .subscription("subscribeToMessages", |t| {
                        t(|ctx, _: ()| {
                                async_stream::stream! {
                                        let auth_user = ctx.get_auth_user().await.unwrap();

                                        let (tx, rx) = broadcast::channel(100);

                                        {
                                                let mut senders = ctx.app_state.message_senders.write().await;
                                                senders.insert(auth_user.id, tx);
                                                tracing::debug!("User {} subscribed to messages", auth_user.id);
                                        };

                                       let stream = BroadcastStream::new(rx);

                                       for await message in stream {
                                               tracing::debug!("Received message: {:?}", message);

                                               match message {
                                                        Ok(message) => yield Some(message),
                                                        Err(_) => yield None
                                                }

                                       }
                                }
                        })
                });

        let message_request_router = rspc::Router::<RequestContext>::new()
                .query("getMessageRequest", |t| {
                        t(|ctx: RequestContext, message_request_id: String| {
                                message_request_controller::get_message_request(ctx, message_request_id)
                        })
                })
                .mutation("createMessageRequest", |t| {
                        t(
                                |ctx: RequestContext, message_request_request: MessageRequestRequestDto| {
                                        message_request_controller::create_message_request(ctx, message_request_request)
                                },
                        )
                })
                .mutation("approveMessageRequest", |t| {
                        t(|ctx: RequestContext, message_request_id: String| {
                                message_request_controller::approve_message_request(ctx, message_request_id)
                        })
                });

        let users_router = rspc::Router::<RequestContext>::new()
                .query("getUser", |t| {
                        t(|ctx: RequestContext, user_id: String| user_controller::get_user(ctx, user_id))
                })
                .mutation("createUser", |t| {
                        t(|ctx: RequestContext, user_request: UserRequestDto| {
                                user_controller::create_user(ctx, user_request)
                        })
                })
                .mutation("createUserProfilePicturePresignedUploadUrl", |t| {
                        t(
                                |ctx: RequestContext, presigned_upload_url_request: PresignedUploadUrlRequestDto| {
                                        user_controller::create_user_profile_picture_presigned_upload_url(
                                                ctx,
                                                presigned_upload_url_request,
                                        )
                                },
                        )
                });

        let user_push_subscriptions_router =
                rspc::Router::<RequestContext>::new().mutation("createUserPushSubscription", |t| {
                        t(
                                |ctx: RequestContext, user_push_subscription_request: UserPushSubscriptionRequestDto| {
                                        user_push_subscription_controller::create_user_push_subscripition(
                                                ctx,
                                                user_push_subscription_request,
                                        )
                                },
                        )
                });

        let router = rspc::Router::<RequestContext>::new()
                .config(Config::new().export_ts_bindings(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gen.ts")))
                .query("version", |t| t(|_, _: ()| "0.1.0"))
                .middleware(|mw| {
                        mw.middleware(|mut mw| async move {
                                let query_str = mw.ctx.parts.uri.query().ok_or(Error::new(
                                        ErrorCode::Unauthorized,
                                        "Missing authorization param".into(),
                                ))?;

                                let query_params: HashMap<_, _> =
                                        url::form_urlencoded::parse(query_str.as_bytes()).into_owned().collect();

                                let authorization = query_params.get("authorization").ok_or(Error::new(
                                        ErrorCode::Unauthorized,
                                        "Missing authorization param".into(),
                                ))?;

                                let token_data = get_cached_token_data(authorization)
                                        .await
                                        .map_err(|_| Error::new(ErrorCode::Unauthorized, "Token invalid".into()))?;

                                mw.ctx.sub = Some(token_data.claims.sub);

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

        let gcp_config = google_cloud_storage::client::ClientConfig::default()
                .with_auth()
                .await
                .expect("Failed to load Google Cloud Storage credentials");

        let app_state = Arc::new(AppState {
                auth_user_cache: Arc::new(RwLock::new(HashMap::new())),
                message_senders: Arc::new(RwLock::new(HashMap::new())),

                id_generator: Arc::new(Mutex::new(SnowflakeIdGenerator::new(1, 1))),

                google_cloud_storage_service: GoogleCloudStorageService::new(
                        google_cloud_storage::client::Client::new(gcp_config),
                ),

                group_repository: GroupRepository::new(pool.clone()),
                message_repository: MessageRepository::new(pool.clone()),
                message_request_repository: MessageRequestRepository::new(pool.clone()),
                user_push_subscription_repository: UserPushSubscriptionRepository::new(pool.clone()),
                user_repository: UserRepository::new(pool.clone()),
        });

        let app = axum::Router::new()
                .route("/health", get(|| async { Json(json!({ "status": "up" })) }))
                .nest(
                        "/rspc",
                        rspc_axum::endpoint(router, move |parts: Parts| RequestContext {
                                parts,
                                sub: None,
                                app_state: Arc::clone(&app_state),
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
