use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, Error, HttpMessage};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use futures_util::FutureExt;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use lazy_static::lazy_static;
use log::error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::errors::problem::Problem;
use crate::AppState;

#[derive(Deserialize, Debug, Clone)]
struct JwkKey {
        kid: String,
        n: String,
        e: String,
        kty: String,
        alg: String,
        r#use: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Claims {
        sub: String,
        exp: usize,
}

struct CachedJwks {
        keys: Vec<JwkKey>,
        last_updated: Instant,
}

struct CachedTokenData {
        data: TokenData<Value>,
        expires_at: Instant,
}

lazy_static! {
        static ref JWKS_CACHE: Arc<RwLock<Option<CachedJwks>>> = Arc::new(RwLock::new(None));
        static ref TOKEN_CACHE: Arc<RwLock<HashMap<String, CachedTokenData>>> = Arc::new(RwLock::new(HashMap::new()));
}

const JWKS_CACHE_DURATION: Duration = Duration::from_secs(3600);
const TOKEN_CACHE_DURATION: Duration = Duration::from_secs(300);

async fn fetch_jwks(jwks_url: &str) -> Result<Vec<JwkKey>, Box<dyn std::error::Error>> {
        let client = Client::new();
        let response = client.get(jwks_url).send().await?.json::<Value>().await?;
        let keys = serde_json::from_value(response["keys"].clone())?;
        Ok(keys)
}

async fn get_cached_jwks() -> Result<Vec<JwkKey>, Box<dyn std::error::Error>> {
        let mut cache = JWKS_CACHE.write().await;

        match &*cache {
                Some(cached_jwks) if cached_jwks.last_updated.elapsed() < JWKS_CACHE_DURATION => {
                        Ok(cached_jwks.keys.clone())
                }
                _ => {
                        let aws_region = env::var("AWS_REGION").expect("AWS_REGION must be set");
                        let aws_cognito_user_pool_id =
                                env::var("AWS_COGNITO_USER_POOL_ID").expect("AWS_COGNITO_USER_POOL_ID must be set");
                        let jwks_url = format!(
                                "https://cognito-idp.{}.amazonaws.com/{}/.well-known/jwks.json",
                                aws_region, aws_cognito_user_pool_id,
                        );
                        let jwks = fetch_jwks(jwks_url.as_str()).await?;
                        *cache = Some(CachedJwks {
                                keys: jwks.clone(),
                                last_updated: Instant::now(),
                        });
                        Ok(jwks)
                }
        }
}

async fn get_cached_token_data(token: &str, jwks: &[JwkKey]) -> Result<TokenData<Value>, jsonwebtoken::errors::Error> {
        let mut cache = TOKEN_CACHE.write().await;

        if let Some(cached_data) = cache.get(token) {
                if Instant::now() < cached_data.expires_at {
                        return Ok(cached_data.data.clone());
                }
        }

        let token_data = decode_token(token, jwks)?;

        cache.insert(
                token.to_string(),
                CachedTokenData {
                        data: token_data.clone(),
                        expires_at: Instant::now() + TOKEN_CACHE_DURATION,
                },
        );

        Ok(token_data)
}

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
{
        type Response = ServiceResponse<B>;
        type Error = Error;
        type Transform = AuthMiddlewareMiddleware<S>;
        type InitError = ();
        type Future = Ready<Result<Self::Transform, Self::InitError>>;

        fn new_transform(&self, service: S) -> Self::Future {
                ok(AuthMiddlewareMiddleware {
                        service: Rc::new(service),
                })
        }
}

pub struct AuthMiddlewareMiddleware<S> {
        service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareMiddleware<S>
where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
{
        type Response = ServiceResponse<B>;
        type Error = Error;
        type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        forward_ready!(service);

        fn call(&self, req: ServiceRequest) -> Self::Future {
                let svc = Rc::clone(&self.service);
                let app_data = req.app_data::<web::Data<AppState>>().unwrap().clone();

                async move {
                        let authorization = req
                                .headers()
                                .get("Authorization")
                                .ok_or(Problem::Unauthorized("Missing authorization header".to_string()))?;

                        let bearer_token = authorization
                                .to_str()
                                .map_err(|_| Problem::Unauthorized("Invalid authorization header".to_string()))?;

                        if !bearer_token.starts_with("Bearer ") {
                                return Err(Error::from(Problem::Unauthorized("Invalid bearer token".to_string())));
                        }

                        let token = &bearer_token[7..];

                        let jwks = get_cached_jwks().await.map_err(|err| {
                                error!("failed to retrieve JWKs: {}", err.to_string());
                                Problem::InternalServerError("failed to retrieve JWKs".to_string())
                        })?;

                        let token_data = get_cached_token_data(token, &jwks).await.map_err(|err| {
                                error!("invalid token: {} {}", token, err.to_string());
                                Problem::Unauthorized("invalid token".to_string())
                        })?;

                        let claims: Claims = serde_json::from_value(token_data.claims.clone()).map_err(|err| {
                                error!("invalid claims for token: {} {}", token, err.to_string());
                                Problem::Unauthorized("invalid claims".to_string())
                        })?;

                        {
                                let mut sub = app_data.sub.lock().unwrap();
                                *sub = Some(claims.sub.clone());
                        }

                        req.extensions_mut().insert(token_data.claims);
                        svc.call(req).await
                }
                .boxed_local()
        }
}

fn decode_token(token: &str, jwks: &[JwkKey]) -> Result<TokenData<Value>, jsonwebtoken::errors::Error> {
        let header = decode_header(token)?;
        let kid = header.kid.ok_or(jsonwebtoken::errors::ErrorKind::InvalidToken)?;

        let jwk = jwks
                .iter()
                .find(|jwk| jwk.kid == kid)
                .ok_or(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat)?;

        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)?;
        let validation = Validation::new(Algorithm::from_str(&jwk.alg)?);

        decode::<Value>(token, &decoding_key, &validation)
}
