use crate::AuthClaims;
use axum::Json;
use axum::extract::{Query, Request};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use url::Url;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;
const SESSION_COOKIE: &str = "quest_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionClaims {
    sub: String,
    name: String,
    email: String,
    groups: Vec<String>,
    exp: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub user_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub token: String,
    pub user_id: String,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub redirect_uri: String,
    pub user_id: Option<String>,
}

pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let claims = if load_test_mode() {
        request
            .headers()
            .get("x-quest-user-id")
            .and_then(|value| value.to_str().ok())
            .map(|user_id| {
                let name = request
                    .headers()
                    .get("x-quest-user-name")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or(user_id);
                auth_claims(user_id, name, Vec::new(), Utc::now().timestamp() + 3600)
            })
    } else {
        None
    }
    .or_else(|| session_token(request.headers()).and_then(|token| verify_token(&token).ok()))
    .or_else(|| {
        allow_dev_auth().then(|| {
            auth_claims(
                "devuser",
                "Development User",
                vec!["O-Quest Admin".to_string()],
                Utc::now().timestamp() + 3600,
            )
        })
    })
    .ok_or(StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

pub async fn create_session(
    Json(payload): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    if !self_registration_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }

    let user_id = registration_user_id(payload.user_id)?;
    let name = payload
        .name
        .as_deref()
        .unwrap_or(&user_id)
        .trim()
        .to_string();
    if name.is_empty() || name.len() > 120 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let expires_at = Utc::now().timestamp() + session_ttl_seconds();
    let claims = SessionClaims {
        email: format!("{user_id}@local.quest"),
        sub: user_id.clone(),
        name,
        groups: Vec::new(),
        exp: expires_at,
    };
    let token = sign_token(&claims).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let headers = session_headers(&token, session_ttl_seconds())?;

    Ok((
        headers,
        Json(CreateSessionResponse {
            token,
            user_id,
            expires_at,
        }),
    ))
}

pub async fn login_redirect(Query(query): Query<LoginQuery>) -> Result<Response, StatusCode> {
    if !self_registration_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }
    if !redirect_is_allowed(&query.redirect_uri) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let user_id = registration_user_id(query.user_id)?;
    let expires_at = Utc::now().timestamp() + session_ttl_seconds();
    let claims = SessionClaims {
        email: format!("{user_id}@local.quest"),
        name: "Orientation Participant".to_string(),
        sub: user_id,
        groups: Vec::new(),
        exp: expires_at,
    };
    let token = sign_token(&claims).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let headers = session_headers(&token, session_ttl_seconds())?;

    Ok((headers, Redirect::temporary(&query.redirect_uri)).into_response())
}

pub async fn logout() -> Result<impl IntoResponse, StatusCode> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite={}; Max-Age=0{}",
            cookie_same_site(),
            secure_cookie_suffix()
        ))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok((headers, StatusCode::NO_CONTENT))
}

fn auth_claims(user_id: &str, name: &str, groups: Vec<String>, exp: i64) -> AuthClaims {
    let now = Utc::now().timestamp();
    AuthClaims {
        iss: "quest-local-auth".to_string(),
        sub: user_id.to_string(),
        aud: "quest".to_string(),
        exp,
        iat: now,
        auth_time: now,
        acr: "local-session".to_string(),
        email: format!("{user_id}@local.quest"),
        email_verified: true,
        name: name.to_string(),
        given_name: name.to_string(),
        preferred_username: user_id.to_string(),
        nickname: user_id.to_string(),
        groups,
    }
}

fn sign_token(claims: &SessionClaims) -> Result<String, Box<dyn std::error::Error>> {
    let payload = serde_json::to_vec(claims)?;
    let encoded_payload = URL_SAFE_NO_PAD.encode(payload);
    let mut mac = HmacSha256::new_from_slice(session_secret().as_bytes())?;
    mac.update(encoded_payload.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("{encoded_payload}.{signature}"))
}

fn verify_token(token: &str) -> Result<AuthClaims, Box<dyn std::error::Error>> {
    let (encoded_payload, encoded_signature) = token.split_once('.').ok_or("bad token")?;
    let signature = URL_SAFE_NO_PAD.decode(encoded_signature)?;
    let mut mac = HmacSha256::new_from_slice(session_secret().as_bytes())?;
    mac.update(encoded_payload.as_bytes());
    mac.verify_slice(&signature)?;

    let claims: SessionClaims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded_payload)?)?;
    if claims.exp <= Utc::now().timestamp() {
        return Err("expired token".into());
    }

    Ok(auth_claims(
        &claims.sub,
        &claims.name,
        claims.groups,
        claims.exp,
    ))
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        && let Some(token) = value.strip_prefix("Session ")
    {
        return Some(token.to_string());
    }

    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == SESSION_COOKIE).then(|| value.to_string())
            })
        })
}

fn validate_user_id(user_id: &str) -> Result<(), StatusCode> {
    if user_id.is_empty()
        || user_id.len() > 80
        || !user_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

fn registration_user_id(requested: Option<String>) -> Result<String, StatusCode> {
    let user_id = if allow_dev_auth() || load_test_mode() {
        requested.unwrap_or_else(|| format!("guest-{}", Uuid::new_v4().simple()))
    } else {
        format!("user-{}", Uuid::new_v4().simple())
    };
    validate_user_id(&user_id)?;
    Ok(user_id)
}

fn session_headers(token: &str, max_age: i64) -> Result<HeaderMap, StatusCode> {
    let mut headers = HeaderMap::new();
    let cookie = format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite={}; Max-Age={max_age}{}",
        cookie_same_site(),
        secure_cookie_suffix()
    );
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(headers)
}

fn redirect_is_allowed(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let origin = match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    };
    let allowed = std::env::var("ALLOWED_REDIRECT_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:1420,http://tauri.localhost,tauri://localhost".to_string()
    });
    allowed
        .split(',')
        .any(|candidate| candidate.trim() == origin)
        || (url.scheme() == "quest" && allowed.split(',').any(|value| value.trim() == "quest://"))
}

fn secure_cookie_suffix() -> &'static str {
    if secure_cookie_enabled() {
        "; Secure"
    } else {
        ""
    }
}

fn secure_cookie_enabled() -> bool {
    std::env::var("SESSION_COOKIE_SECURE").as_deref() == Ok("true")
}

fn cookie_same_site() -> &'static str {
    match std::env::var("SESSION_COOKIE_SAME_SITE")
        .unwrap_or_else(|_| "Lax".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "strict" => "Strict",
        "none" if secure_cookie_enabled() => "None",
        _ => "Lax",
    }
}

fn session_secret() -> String {
    let secret = std::env::var("SESSION_SECRET").unwrap_or_else(|_| {
        if cfg!(test) || allow_dev_auth() || load_test_mode() {
            "development-only-session-secret-change-me".to_string()
        } else {
            panic!("SESSION_SECRET must be set")
        }
    });
    assert!(
        secret.len() >= 32,
        "SESSION_SECRET must contain at least 32 bytes"
    );
    secret
}

fn session_ttl_seconds() -> i64 {
    std::env::var("SESSION_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(12 * 60 * 60)
}

fn allow_dev_auth() -> bool {
    cfg!(debug_assertions) && std::env::var("ALLOW_DEV_AUTH").as_deref() == Ok("true")
}

fn load_test_mode() -> bool {
    std::env::var("LOAD_TEST_MODE").as_deref() == Ok("true")
}

fn self_registration_enabled() -> bool {
    std::env::var("ALLOW_SELF_REGISTRATION").as_deref() == Ok("true")
        || allow_dev_auth()
        || load_test_mode()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(exp: i64) -> SessionClaims {
        SessionClaims {
            sub: "test-user".to_string(),
            name: "Test User".to_string(),
            email: "test-user@local.quest".to_string(),
            groups: vec!["participants".to_string()],
            exp,
        }
    }

    #[test]
    fn signed_session_round_trips_and_rejects_tampering() {
        let token = sign_token(&claims(Utc::now().timestamp() + 60)).unwrap();
        let verified = verify_token(&token).unwrap();
        assert_eq!(verified.sub, "test-user");
        assert_eq!(verified.name, "Test User");

        let mut tampered = token.into_bytes();
        let last = tampered.last_mut().unwrap();
        *last = if *last == b'A' { b'B' } else { b'A' };
        assert!(verify_token(&String::from_utf8(tampered).unwrap()).is_err());
    }

    #[test]
    fn expired_session_is_rejected() {
        let token = sign_token(&claims(Utc::now().timestamp() - 1)).unwrap();
        assert!(verify_token(&token).is_err());
    }

    #[test]
    fn default_redirects_accept_local_web_and_tauri_origins() {
        assert!(redirect_is_allowed("http://localhost:1420/profile"));
        assert!(redirect_is_allowed("tauri://localhost/profile"));
        assert!(!redirect_is_allowed("https://retired.example/profile"));
    }
}
