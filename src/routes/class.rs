use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use tokio::try_join;

use crate::{auth::UserClient, AppState, Error};

pub async fn classes(
    UserClient(client): UserClient,
    State(state): State<AppState>,
) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    let classes = client.courses().list().doit().await?;
    context.insert("classes", &classes.1.courses);
    Ok(Html(state.tera.render("classes.jinja", &context)?))
}

pub async fn class(
    UserClient(client): UserClient,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(pages): Query<PaginationQuery>,
) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    let req_general = client.courses().get(&id);
    let mut req_work = client
        .courses()
        .course_work_list(&id)
        .page_size(10)
        .param("fields", "nextPageToken,courseWork(id,title)");
    if let Some(page) = pages.page {
        req_work = req_work.page_token(&page);
        context.insert("is_first_page", &false);
    } else {
        context.insert("is_first_page", &true);
    }
    let (general, work) = try_join!(req_general.doit(), req_work.doit())?;
    context.insert("class", &general.1);
    context.insert("coursework", &work.1);
    context.insert("pagination_token", &work.1.next_page_token);
    Ok(Html(state.tera.render("class.jinja", &context)?))
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    page: Option<String>,
}
