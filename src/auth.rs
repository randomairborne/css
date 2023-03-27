use axum::{extract::FromRequestParts, http::request::Parts};
use oauth2::{reqwest::async_http_client, RefreshToken, TokenResponse};
use tower_cookies::Cookies;

use crate::{AppState, Error};

pub struct AccessToken(pub String);

#[axum::async_trait]
impl FromRequestParts<AppState> for AccessToken {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookies = match Cookies::from_request_parts(parts, state).await {
            Ok(v) => v,
            Err(e) => return Err(Error::Extractor(e.1)),
        };
        let cookies = cookies.private(&state.key);
        let access_token = if let Some(access_token) = cookies.get("access") {
            access_token.value().to_string()
        } else {
            let Some(refresh_token) = cookies.get("refresh") else {
                return Err(Error::NoToken);
            };
            let access = state
                .oauth
                .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
                .request_async(async_http_client)
                .await?;
            access.access_token().secret().to_string()
        };
        Ok(Self(access_token))
    }
}
