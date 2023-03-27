use axum::{extract::State, response::Html};

use crate::{auth::AccessToken, AppState, Error};

pub async fn classes(
    AccessToken(token): AccessToken,
    State(state): State<AppState>,
) -> Result<Html<String>, Error> {
    let client = classroom::Classroom::new(state.client, token);
    let mut context = tera::Context::new();
    let classes = client.courses().list().doit().await?;
    context.insert("classes", &classes.1.courses);
    Ok(Html(state.tera.render("classes.jinja", &context)?))
}
