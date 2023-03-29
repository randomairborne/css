use axum::{extract::State, response::Html};
use classroom::{
    api::{Course, Date, TimeOfDay},
    chrono::{NaiveDate, NaiveDateTime, NaiveTime},
    Classroom,
};
use tokio::task::JoinSet;

use crate::{auth::UserClient, state::ClassroomHyperClient, AppState, Error};

pub async fn todo(
    UserClient(client): UserClient,
    State(state): State<AppState>,
) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    let courses = client
        .courses()
        .list()
        .param("fields", "courses(id,name)")
        .doit()
        .await?;
    let mut assignment_list: Vec<Todo> = Vec::new();
    let mut lister_joins = JoinSet::new();
    for course in courses
        .1
        .courses
        .ok_or(Error::MissingField("courses.list.courses"))?
    {
        lister_joins.spawn(todo_get_course(client.clone(), course));
    }
    while let Some(res) = lister_joins.join_next().await {
        assignment_list.append(&mut res??);
    }
    context.insert("todos", &assignment_list);
    Ok(Html(state.tera.render("todo.jinja", &context)?))
}

#[derive(serde::Serialize)]
struct Todo {
    class_name: String,
    id: String,
    description: Option<String>,
    name: String,
    due: Option<NaiveDateTime>,
}

async fn todo_get_course<'a>(
    client: Classroom<ClassroomHyperClient>,
    course: Course,
) -> Result<Vec<Todo>, Error> {
    let course_id = course
        .id
        .ok_or(Error::MissingField("courses.list.courses[].id"))?;
    let class_name = course
        .name
        .ok_or(Error::MissingField("courses.list.courses[].name"))?;
    let assignments = client
        .courses()
        .course_work_list(&course_id)
        .order_by("dueDate desc")
        .param(
            "fields",
            "courseWork(dueDate,dueTime,id,title,alternateLink)",
        )
        .doit()
        .await?
        .1
        .course_work
        .ok_or(Error::MissingField("courses.courseWork"))?;
    let mut todos = Vec::with_capacity(assignments.len());
    for assignment in assignments {
        let due = assignment_due_date(assignment.due_date, assignment.due_time);
        let id = assignment
            .id
            .ok_or(Error::MissingField("courses.courseWork[].id"))?;
        let name = assignment
            .title
            .ok_or(Error::MissingField("courses.courseWork[].title"))?;
        let todo = Todo {
            class_name: class_name.clone(),
            id,
            description: assignment.description,
            name,
            due,
        };
        todos.push(todo);
    }
    Ok(todos)
}

fn assignment_due_date(date: Option<Date>, time: Option<TimeOfDay>) -> Option<NaiveDateTime> {
    let (date, time) = (date?, time?);
    let ndt = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(
            date.year?,
            date.month?.try_into().ok()?,
            date.day?.try_into().ok()?,
        )?,
        NaiveTime::from_hms_opt(
            time.hours?.try_into().ok()?,
            time.minutes?.try_into().ok()?,
            time.seconds?.try_into().ok()?,
        )?,
    );
    Some(ndt)
}
