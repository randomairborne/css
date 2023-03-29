use axum::{extract::State, response::Html};

use crate::{AppState, Error};

#[allow(clippy::unused_async)]
pub async fn about(State(state): State<AppState>) -> Result<Html<String>, Error> {
    Ok(Html(
        state.tera.render("index.jinja", &tera::Context::new())?,
    ))
}

#[allow(clippy::unused_async)]
pub async fn terms(State(state): State<AppState>) -> Result<Html<String>, Error> {
    Ok(Html(
        state.tera.render("terms.jinja", &tera::Context::new())?,
    ))
}

#[allow(clippy::unused_async)]
pub async fn privacy(State(state): State<AppState>) -> Result<Html<String>, Error> {
    Ok(Html(
        state.tera.render("privacy.jinja", &tera::Context::new())?,
    ))
}
