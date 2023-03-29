use axum::extract::{Query, State};
use axum::response::Redirect;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge, Scope, TokenResponse};
use tower_cookies::cookie::time::OffsetDateTime;
use tower_cookies::{Cookie, Cookies};

use crate::{AppState, Error};

const SCOPES: [&str; 11] = [
    "userinfo.email",
    "userinfo.profile",
    "classroom.announcements",
    "classroom.coursework.me",
    "classroom.courseworkmaterials",
    "classroom.topics",
    "classroom.guardianlinks.me.readonly",
    "classroom.courses",
    "classroom.addons.student",
    "classroom.profile.emails",
    "classroom.profile.photos",
];

pub async fn redirect(State(state): State<AppState>) -> Result<Redirect, Error> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let scopes = SCOPES
        .iter()
        .map(|scope| Scope::new(format!("https://www.googleapis.com/auth/{scope}")));
    let (auth_url, csrf_token) = state
        .oauth
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge)
        .add_scopes(scopes)
        .url();
    state
        .tokens
        .write()
        .await
        .insert(csrf_token.secret().to_string(), pkce_verifier);
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;
        state.tokens.write().await.remove(csrf_token.secret());
    });
    Ok(Redirect::to(auth_url.as_str()))
}

pub async fn set_tokens(
    State(state): State<AppState>,
    Query(query): Query<SetIdQuery>,
    encrypted_cookies: Cookies,
) -> Result<Redirect, Error> {
    let pkce_verifier = state
        .tokens
        .write()
        .await
        .remove(&query.state)
        .ok_or(Error::InvalidState)?;
    let token_result = state
        .oauth
        .exchange_code(AuthorizationCode::new(query.code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await
        .map_err(|_| Error::CodeExchangeFailed)?;
    let access = token_result.access_token().secret().clone();
    let private_cookies = encrypted_cookies.private(&state.key);
    let mut access_cookie = Cookie::new("access", access);
    access_cookie.set_expires(
        OffsetDateTime::now_utc().saturating_add(
            token_result
                .expires_in()
                .unwrap_or_else(|| std::time::Duration::from_secs(3600))
                .try_into()?,
        ),
    );
    access_cookie.set_path("/");
    private_cookies.add(access_cookie);
    if let Some(refresh) = token_result.refresh_token().map(|v| v.secret().clone()) {
        let mut refresh_cookie = Cookie::new("refresh", refresh);
        refresh_cookie.set_path("/");
        private_cookies.add(refresh_cookie);
    }
    Ok(Redirect::to("/classes"))
}

#[derive(serde::Deserialize)]
pub struct SetIdQuery {
    code: String,
    state: String,
}
