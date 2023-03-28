use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use classroom::api::CourseWork;
use tokio::try_join;

use crate::{auth::UserClient, AppState, Error};

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    page: Option<String>,
}

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
    }
    let (general, work) = try_join!(req_general.doit(), req_work.doit())?;
    context.insert("class", &general.1);
    context.insert("coursework", &work.1);
    context.insert("pagination_token", &work.1.next_page_token);
    Ok(Html(state.tera.render("class.jinja", &context)?))
}

pub async fn todo(
    UserClient(client): UserClient,
    State(state): State<AppState>,
) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    let courses = client.courses().list().doit().await?;
    let mut assignment_list: Vec<CourseWork> = Vec::new();
    for course in courses
        .1
        .courses
        .ok_or(Error::MissingField("courses.list.courses"))?
    {
        let id = course
            .id
            .ok_or(Error::MissingField("courses.list.courses.[list].id"))?;
        if let Some(mut assignments) = client
            .courses()
            .course_work_list(&id)
            .page_size(0)
            .order_by("dueDate desc")
            .doit()
            .await?
            .1
            .course_work
        {
            assignment_list.append(&mut assignments);
        }
    }
    context.insert("assignments", &assignment_list);
    Ok(Html(state.tera.render("class.jinja", &context)?))
}
