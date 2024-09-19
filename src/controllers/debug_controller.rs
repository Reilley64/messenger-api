use actix_web::web::Json;
use actix_web::Responder;
use apistos::api_operation;
use serde_json::json;

#[api_operation(operation_id = "get_health")]
pub async fn health() -> impl Responder {
        Json(json!({ "status": "UP" }))
}
