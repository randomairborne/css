use axum::{
    extract::{Path, State},
    response::Html,
};
use tokio::try_join;

use crate::{auth::UserClient, AppState, Error};

pub async fn assignment(
    UserClient(client): UserClient,
    State(state): State<AppState>,
    Path((course_id, id)): Path<(String, String)>,
) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    let req_general = client.courses().get(&course_id).param("fields", "id,name");
    let req_work = client
        .courses()
        .course_work_get(&course_id, &id)
        .param("fields", "nextPageToken,courseWork(id,title)");
    let (general, work) = try_join!(req_general.doit(), req_work.doit())?;
    context.insert("class", &general.1);
    context.insert("coursework", &work.1);
    Ok(Html(state.tera.render("assignment.jinja", &context)?))
}
