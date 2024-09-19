use std::fmt::{Display, Formatter};

use actix_web::{http, HttpResponse, ResponseError};
use apistos::ApiErrorComponent;
use serde::{self, Deserialize, Serialize};
use serde_json::json;

const CONTENT_TYPE: &str = "application/problem+json";
const TYPE_URL: &str = "https://api.messenger.reilley.dev/problems";

#[derive(Serialize, Deserialize, ApiErrorComponent, Debug, Clone)]
#[openapi_error(
        status(code = 400),
        status(code = 409),
        status(code = 500),
        status(code = 404),
        status(code = 401)
)]
pub enum Problem {
        BadRequest(String),
        Conflict(String),
        InternalServerError(String),
        NotFound(String),
        Unauthorized(String),
}

impl Display for Problem {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{self}")
        }
}

impl ResponseError for Problem {
        fn error_response(&self) -> HttpResponse {
                match self {
                        Problem::BadRequest(detail) => HttpResponse::build(http::StatusCode::BAD_REQUEST)
                                .content_type(CONTENT_TYPE)
                                .json(json!({
                                    "type": format!("{}/{}", TYPE_URL, "bad-request"),
                                    "title": "Bad Request",
                                    "status": http::StatusCode::BAD_REQUEST.as_u16(),
                                    "detail": detail,
                                })),
                        Problem::Conflict(detail) => HttpResponse::build(http::StatusCode::CONFLICT)
                                .content_type(CONTENT_TYPE)
                                .json(json!({
                                    "type": format!("{}/{}", TYPE_URL, "conflict"),
                                    "title": "Conflict",
                                    "status": http::StatusCode::CONFLICT.as_u16(),
                                    "detail": detail,
                                })),
                        Problem::InternalServerError(detail) => {
                                HttpResponse::build(http::StatusCode::INTERNAL_SERVER_ERROR)
                                        .content_type(CONTENT_TYPE)
                                        .json(json!({
                                            "type": format!("{}/{}", TYPE_URL, "internal-server-error"),
                                            "title": "Internal Sever Error",
                                            "status": http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                                            "detail": detail,
                                        }))
                        }
                        Problem::NotFound(detail) => HttpResponse::build(http::StatusCode::NOT_FOUND)
                                .content_type(CONTENT_TYPE)
                                .json(json!({
                                    "type": format!("{}/{}", TYPE_URL, "not-found"),
                                    "title": "Not Found",
                                    "status": http::StatusCode::NOT_FOUND.as_u16(),
                                    "detail": detail,
                                })),
                        Problem::Unauthorized(detail) => HttpResponse::build(http::StatusCode::UNAUTHORIZED)
                                .content_type(CONTENT_TYPE)
                                .json(json!({
                                    "type": format!("{}/{}", TYPE_URL, "unauthorized"),
                                    "title": "Unauthorized",
                                    "status": http::StatusCode::UNAUTHORIZED.as_u16(),
                                    "detail": detail,
                                })),
                }
        }
}
