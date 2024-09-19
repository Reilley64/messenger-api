use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use apistos::app::OpenApiWrapper;
use apistos::info::Info;
use apistos::server::Server;
use apistos::spec::Spec;
use apistos::web::{get, post, resource, tagged_scope};
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use env_logger::Env;
use messenger_api::controllers::{
        auth_controller, debug_controller, group_controller, message_controller, message_request_controller,
        user_controller,
};
use messenger_api::middleware::authorization::AuthMiddleware;
use messenger_api::repositories::group_repository::GroupRepository;
use messenger_api::repositories::message_repository::MessageRepository;
use messenger_api::repositories::message_request_repository::MessageRequestRepository;
use messenger_api::repositories::user_repository::UserRepository;
use messenger_api::AppState;
use snowflake::SnowflakeIdGenerator;
use std::env;
use std::sync::Mutex;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[actix_web::main]
async fn main() -> std::io::Result<()> {
        dotenv().ok();

        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = r2d2::Pool::builder().build(manager).expect("failed to create pool.");
        pool.get()
                .expect("failed to get connection for migrations")
                .run_pending_migrations(MIGRATIONS)
                .expect("failed to run migrations");

        let app_state = web::Data::new(AppState {
                id_generator: Mutex::new(SnowflakeIdGenerator::new(1, 1)),

                group_repository: GroupRepository::new(pool.clone()),
                message_repositoy: MessageRepository::new(pool.clone()),
                message_request_repository: MessageRequestRepository::new(pool.clone()),
                user_repository: UserRepository::new(pool.clone()),

                sub: Mutex::new(None),
        });

        let host = env::var("SERVER_HOST").unwrap_or("127.0.0.1".to_string());
        let port = env::var("SERVER_PORT")
                .unwrap_or("8080".to_string())
                .parse()
                .expect("failed to parse port");
        HttpServer::new(move || {
                let spec = Spec {
                        info: Info {
                                title: "Messenger API".to_string(),
                                version: "1.0.0".to_string(),
                                ..Default::default()
                        },
                        servers: vec![Server {
                                url: "https://api.messenger.reilley.dev".to_string(),
                                ..Default::default()
                        }],
                        ..Default::default()
                };

                App::new()
                        .document(spec)
                        .app_data(app_state.clone())
                        .wrap(Logger::default())
                        .wrap(Cors::permissive())
                        .service(resource("/health").route(get().to(debug_controller::health)))
                        .service(
                                tagged_scope("v1", vec!["v1"])
                                        .wrap(AuthMiddleware)
                                        .service(
                                                tagged_scope("user", vec!["auth-rest-controller"]).service(
                                                        resource("/").route(get().to(auth_controller::get_auth_user)),
                                                ),
                                        )
                                        .service(
                                                tagged_scope("groups", vec!["group-rest-controller"])
                                                        .service(
                                                                resource("/{group_id}")
                                                                        .route(get().to(group_controller::get_group)),
                                                        )
                                                        .service(
                                                                resource("/{group_id}/messages")
                                                                        .route(get().to(
                                                                                group_controller::get_group_messages,
                                                                        ))
                                                                        .route(post().to(
                                                                                group_controller::create_group_message,
                                                                        )),
                                                        ),
                                        )
                                        .service(tagged_scope("/messages", vec!["message-rest-controller"]).service(
                                                resource("/messages").route(get().to(message_controller::get_messages)),
                                        ))
                                        .service(
                                                tagged_scope(
                                                        "/message-requests",
                                                        vec!["message-request-rest-controller"],
                                                )
                                                .service(resource("/").route(
                                                        post().to(message_request_controller::create_message_request),
                                                ))
                                                .service(resource("/{message_request_id}").route(
                                                        get().to(message_request_controller::get_message_request),
                                                ))
                                                .service(
                                                        resource("/{message_request_id}/approve").route(post().to(
                                                                message_request_controller::approve_message_request,
                                                        )),
                                                ),
                                        )
                                        .service(
                                                tagged_scope("/users", vec!["user-rest-controller"])
                                                        .service(
                                                                resource("/")
                                                                        .route(post().to(user_controller::create_user)),
                                                        )
                                                        .service(
                                                                resource("/{user_id}")
                                                                        .route(get().to(user_controller::get_user)),
                                                        ),
                                        ),
                        )
                        .build("/openapi.json")
        })
        .bind((host, port))?
        .run()
        .await
}
