use axum::{extract::{State, Path}, response::Html};
use tokio::try_join;

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

pub async fn class(
    AccessToken(token): AccessToken,
    State(state): State<AppState>,
    Path(id): Path<String>
) -> Result<Html<String>, Error> {
    let client = classroom::Classroom::new(state.client, token);
    let mut context = tera::Context::new();
    let general = client.courses().get(&id).doit();
    let work = client.courses().course_work_list(&id).doit();
    let (general, work) = try_join!(general, work)?;
    context.insert("class", &general.1);
    context.insert("coursework", &work.1);
    Ok(Html(state.tera.render("class.jinja", &context)?))
}
