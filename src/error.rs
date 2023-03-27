use std::sync::Arc;

use once_cell::sync::OnceCell;

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};

pub static ERROR_TERA: OnceCell<Arc<tera::Tera>> = OnceCell::new();

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Tera error: {0:?}")]
    Tera(#[from] tera::Error),
    #[error("google-classroom1 error: {0:?}")]
    GoogleClassroom(#[from] google_classroom1::Error),
    #[error("SerdeJson error: {0:?}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("OAuth error")]
    OAuth(#[from] oauth2::basic::BasicRequestTokenError<oauth2::reqwest::Error<reqwest::Error>>),
    #[error("Extractor error: {0}")]
    Extractor(&'static str),
    #[error("Tower-Cookies time error: {0}")]
    DurationOutOfRange(#[from] tower_cookies::cookie::time::error::ConversionRange),
    #[error("Invalid OAuth State")]
    InvalidState,
    #[error("OAuth Code Exchange Failed")]
    CodeExchangeFailed,
    #[error("No token found - please reauthenticate")]
    NoToken,
    #[error("Once cell uninitialized, please make an issue")]
    UninitializedOnceCell,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        if matches!(self, Self::NoToken) {
            return Redirect::to("/oauth").into_response();
        }
        let Some(tera) = ERROR_TERA.get() else {
            return Self::UninitializedOnceCell.to_ugly_response()
        };
        let mut context = tera::Context::new();
        context.insert("error", &self.to_string());
        match tera.render("error.jinja", &context) {
            Ok(v) => Html(v).into_response(),
            Err(e) => Self::Tera(e).to_ugly_response(),
        }
    }
}
impl Error {
    pub fn to_ugly_response(&self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "There was an error while processing your request.
Additionally, there was an error while trying to use \
an Error to nicely display the error:
{self:#?}"
            ),
        )
            .into_response()
    }
}
