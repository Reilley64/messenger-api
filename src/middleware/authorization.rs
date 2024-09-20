use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, Error};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use futures_util::FutureExt;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use lazy_static::lazy_static;
use log::{error, info};
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Claims {
        sub: String,
        exp: usize,
}

struct CachedTokenData {
        data: TokenData<Claims>,
        expires_at: Instant,
}

lazy_static! {
        static ref JWKS_CACHE: Arc<RwLock<HashMap<String, JwkKey>>> = Arc::new(RwLock::new(HashMap::new()));
        static ref TOKEN_CACHE: Arc<RwLock<HashMap<String, CachedTokenData>>> = Arc::new(RwLock::new(HashMap::new()));
}

async fn fetch_jwks(jwks_url: &str) -> Result<Vec<JwkKey>, Box<dyn std::error::Error>> {
        info!("fetching jwks");
        let client = Client::new();
        let response = client.get(jwks_url).send().await?.json::<Value>().await?;
        let keys = serde_json::from_value(response["keys"].clone())?;
        Ok(keys)
}

async fn refresh_jwks_cache() -> Result<(), Box<dyn std::error::Error>> {
        let aws_region = env::var("AWS_REGION").expect("AWS_REGION must be set");
        let aws_cognito_user_pool_id =
                env::var("AWS_COGNITO_USER_POOL_ID").expect("AWS_COGNITO_USER_POOL_ID must be set");

        let jwks_url = format!(
                "https://cognito-idp.{}.amazonaws.com/{}/.well-known/jwks.json",
                aws_region, aws_cognito_user_pool_id,
        );
        let jwks = fetch_jwks(&jwks_url).await?;

        let mut cache = JWKS_CACHE.write().await;
        cache.clear();
        for jwk in jwks {
                cache.insert(jwk.kid.clone(), jwk);
        }
        Ok(())
}

async fn get_jwk(kid: &str) -> Result<JwkKey, Box<dyn std::error::Error>> {
        let cache = JWKS_CACHE.read().await;
        if let Some(jwk) = cache.get(kid) {
                return Ok(jwk.clone());
        }
        drop(cache);

        refresh_jwks_cache().await?;

        let cache = JWKS_CACHE.read().await;
        cache.get(kid).cloned().ok_or_else(|| "JWK not found".into())
}

async fn decode_token(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
        let aws_region = env::var("AWS_REGION").expect("AWS_REGION must be set");
        let aws_cognito_user_pool_id =
                env::var("AWS_COGNITO_USER_POOL_ID").expect("AWS_COGNITO_USER_POOL_ID must be set");

        let header = decode_header(token)?;
        let kid = header.kid.ok_or(jsonwebtoken::errors::ErrorKind::InvalidToken)?;

        let jwk = get_jwk(&kid)
                .await
                .map_err(|_| jsonwebtoken::errors::ErrorKind::InvalidKeyFormat)?;

        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)?;
        let mut validation = Validation::new(Algorithm::from_str(&jwk.alg)?);
        validation.set_issuer(&[format!(
                "https://cognito-idp.{}.amazonaws.com/{}",
                aws_region, aws_cognito_user_pool_id
        )]);

        decode::<Claims>(token, &decoding_key, &validation)
}

async fn get_cached_token_data(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
        let cache = TOKEN_CACHE.read().await;
        if let Some(cached_data) = cache.get(token) {
                if Instant::now() < cached_data.expires_at {
                        return Ok(cached_data.data.clone());
                }
        }
        drop(cache);

        let token_data = decode_token(token).await?;
        let expires_at = SystemTime::UNIX_EPOCH + Duration::from_secs(token_data.claims.exp as u64);
        let expires_at = Instant::now()
                + expires_at
                        .duration_since(SystemTime::now())
                        .unwrap_or(Duration::from_secs(0));

        let mut cache = TOKEN_CACHE.write().await;
        cache.insert(
                token.to_string(),
                CachedTokenData {
                        data: token_data.clone(),
                        expires_at,
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

                        let token_data = get_cached_token_data(token).await.map_err(|err| {
                                error!("invalid token: {} {}", token, err.to_string());
                                Problem::Unauthorized("Invalid token".to_string())
                        })?;

                        {
                                let mut sub = app_data.sub.lock().unwrap();
                                *sub = Some(token_data.claims.sub.clone());
                        }

                        svc.call(req).await
                }
                .boxed_local()
        }
}
