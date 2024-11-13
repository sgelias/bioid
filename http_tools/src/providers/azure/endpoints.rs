use super::{
    functions::oauth_client,
    models::{
        AzureTokenResponse, CsrfTokenClaims, ExtraTokenFields, QueryCode,
    },
};
use crate::{models::auth_config::AuthConfig, utils::HttpJsonResponse};

use actix_web::{get, web, HttpResponse};
use chrono::Utc;
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, DecodingKey, EncodingKey, Header,
    Validation,
};
use myc_config::optional_config::OptionalConfig;
use oauth2::{
    reqwest::async_http_client, AuthorizationCode, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::json;
use tracing::error;

pub fn configure(conf: &mut web::ServiceConfig) {
    conf.service(login).service(callback);
}

#[get("/login")]
pub async fn login(config: web::Data<AuthConfig>) -> HttpResponse {
    let config = if let OptionalConfig::Enabled(config) =
        config.get_ref().azure.to_owned()
    {
        config
    } else {
        error!("Azure OAuth is disabled");
        return HttpResponse::InternalServerError().finish();
    };

    let client = match oauth_client(config.to_owned()) {
        Ok(client) => client,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err.msg()));
        }
    };

    let csrf: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

    let claims = CsrfTokenClaims {
        exp: (Utc::now().timestamp() + config.csrf_token_expiration) as usize,
        csrf: csrf.to_owned(),
        code_verifier: verifier.secret().to_owned(),
    };

    let jwt_secret = match config.jwt_secret.get_or_error() {
        Ok(secret) => secret,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
    };

    let csrf_token_jwt = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    ) {
        Ok(token) => token,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .body("Error on CSRF token generation");
        }
    };

    let state_fn = move || CsrfToken::new(csrf_token_jwt.clone());

    let (authorize_url, _) = client
        .authorize_url(state_fn)
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(challenge)
        .url();

    HttpResponse::Ok().json(json!({
        "authorize_url": authorize_url.to_string()
    }))
}

#[get("/callback")]
pub async fn callback(
    query: web::Query<QueryCode>,
    config: web::Data<AuthConfig>,
) -> HttpResponse {
    if let Some(err) = query.error.to_owned() {
        let error_description =
            query.error_description.to_owned().unwrap_or("".to_owned());

        error!("Error on callback: {err}: {error_description}");

        return HttpResponse::BadRequest().json(json!({
            "error": err,
            "description": error_description,
            "step": "callback"
        }));
    }

    let code = match query.code.to_owned() {
        Some(code) => code,
        None => {
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message("Code not found".to_owned()),
            );
        }
    };

    let config = if let OptionalConfig::Enabled(config) =
        config.get_ref().azure.to_owned()
    {
        config
    } else {
        error!("Azure OAuth is disabled");
        return HttpResponse::InternalServerError().finish();
    };

    let client = match oauth_client(config.to_owned()) {
        Ok(client) => client,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err.msg()));
        }
    };

    let jwt_secret = match config.jwt_secret.get_or_error() {
        Ok(secret) => secret,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
    };

    //
    // Decode CSRF Token
    //
    // If the token is invalid or expired, return an error
    //
    let csrf_claims = match decode::<CsrfTokenClaims>(
        &query.state.to_owned(),
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(token_data) => token_data.claims,
        Err(err) => match *err.kind() {
            ErrorKind::ExpiredSignature => {
                return HttpResponse::Unauthorized().json(
                    HttpJsonResponse::new_message(
                        "CSRF Token expired".to_owned(),
                    ),
                );
            }
            _ => {
                return HttpResponse::Unauthorized().json(json!({
                    "error": "Invalid CSRF Token",
                    "description": err.to_string(),
                    "step": "csrf"
                }));
            }
        },
    };

    let code = AuthorizationCode::new(code.clone());

    match client
        .exchange_code(code)
        .set_pkce_verifier(PkceCodeVerifier::new(csrf_claims.code_verifier))
        .request_async(async_http_client)
        .await
    {
        Ok(token) => {
            let access_token = token.access_token();

            let token_response = AzureTokenResponse::new(
                access_token.to_owned(),
                token.token_type().to_owned(),
                ExtraTokenFields {},
            );

            HttpResponse::Ok().json(token_response)
        }
        Err(err) => {
            match err {
                oauth2::RequestTokenError::ServerResponse(response) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": response.error().to_string(),
                        "error_description": response.error_description(),
                        "step": "token_exchange_server"
                    }))
                }
                oauth2::RequestTokenError::Request(ref err) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": err.to_string(),
                        "step": "token_exchange_request"
                    }))
                }
                ref response => {
                    error!(
                        "Error on token exchange: {:?}",
                        response.to_string()
                    );
                }
            }

            HttpResponse::InternalServerError().json(
                HttpJsonResponse::new_message(format!(
                    "Error on token exchange: {err}"
                )),
            )
        }
    }
}