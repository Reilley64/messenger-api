use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

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
pub struct Claims {
        pub sub: String,
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
        info!("Starting to fetch JWKS from URL: {}", jwks_url);

        let client = Client::new();

        debug!("Sending GET request to JWKS URL");
        let response = match client.get(jwks_url).send().await {
                Ok(resp) => {
                        info!("Received response from JWKS URL");
                        resp
                }
                Err(e) => {
                        error!("Failed to send request to JWKS URL: {}", e);
                        return Err(Box::new(e));
                }
        };

        debug!("Attempting to parse response as JSON");
        let json_value: Value = match response.json().await {
                Ok(json) => {
                        info!("Successfully parsed response as JSON");
                        json
                }
                Err(e) => {
                        error!("Failed to parse response as JSON: {}", e);
                        return Err(Box::new(e));
                }
        };

        debug!("Extracting 'keys' from JSON response");
        let keys_value = json_value.get("keys").ok_or_else(|| {
                let err = "'keys' field not found in JWKS response";
                error!("{}", err);
                std::io::Error::new(std::io::ErrorKind::InvalidData, err)
        })?;

        debug!("Deserializing 'keys' into Vec<JwkKey>");
        let keys = match serde_json::from_value::<Vec<JwkKey>>(keys_value.clone()) {
                Ok(k) => {
                        info!("Successfully deserialized {} keys", k.len());
                        k
                }
                Err(e) => {
                        error!("Failed to deserialize 'keys': {}", e);
                        return Err(Box::new(e));
                }
        };

        info!("Successfully fetched and parsed JWKS");
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

pub async fn get_cached_token_data(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
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
