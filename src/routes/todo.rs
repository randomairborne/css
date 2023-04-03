use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::Html,
};
use classroom::{
    api::{Course, StudentSubmission},
    Classroom,
};
use tokio::{task::JoinSet, try_join};

use crate::{auth::UserClient, state::ClassroomHyperClient, AppState, Error};

pub async fn todos_all(
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
        lister_joins.spawn(get_course(client.clone(), course));
    }
    while let Some(res) = lister_joins.join_next().await {
        assignment_list.append(&mut res??);
    }
    context.insert("todos", &assignment_list);
    Ok(Html(state.tera.render("todo.jinja", &context)?))
}

pub async fn todos_for_class(
    UserClient(client): UserClient,
    State(state): State<AppState>,
    Path(course_id): Path<String>,
) -> Result<Html<String>, Error> {
    let mut context = tera::Context::new();
    let course = client
        .courses()
        .get(&course_id)
        .param("fields", "id,name")
        .doit()
        .await?
        .1;
    let assignment_list: Vec<Todo> = get_course(client, course).await?;
    context.insert("todos", &assignment_list);
    Ok(Html(state.tera.render("todo.jinja", &context)?))
}

#[derive(serde::Serialize)]
struct Todo {
    class_name: String,
    id: String,
    description: Option<String>,
    name: Option<String>,
    late: bool,
}

async fn get_course(
    client: Classroom<ClassroomHyperClient>,
    course: Course,
) -> Result<Vec<Todo>, Error> {
    let course_id = course
        .id
        .ok_or(Error::MissingField("courses.list.courses[].id"))?;
    let class_name = course
        .name
        .ok_or(Error::MissingField("courses.list.courses[].name"))?;
    let courses = client.courses();
    let submissions_req = courses
        .course_work_student_submissions_list(&course_id, "-")
        .param(
            "fields",
            "studentSubmissions(courseWorkId,state,late,id,courseWorkId,assignedGrade)",
        )
        .doit();
    let course_work_req = courses
        .course_work_list(&course_id)
        .param("fields", "courseWork(id,title,description)")
        .doit();
    let (course_work_resp, submissions_resp) = try_join!(course_work_req, submissions_req)?;
    let submissions = submissions_resp
        .1
        .student_submissions
        .ok_or(Error::MissingField(
            "courses.courseWork.studentSubmissions[]",
        ))?;
    let course_works = course_work_resp
        .1
        .course_work
        .ok_or(Error::MissingField("courses.courseWork.studentSubmissions"))?;
    let mut title_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    for course in course_works {
        if let Some(id) = course.id {
            title_map.insert(id, (course.title, course.description));
        }
    }
    let mut todos = Vec::new();
    let submissions: Vec<StudentSubmission> =
        submissions.into_iter().filter(is_incomplete).collect();
    for submission in submissions.into_iter() {
        let late = is_late(&submission);
        let id = submission.id.ok_or(Error::MissingField(
            "courses.courseWork.studentSubmissions[].id",
        ))?;
        let course_id = submission.course_work_id.ok_or(Error::MissingField(
            "courses.courseWork.studentSubmissions[].courseWorkId",
        ))?;
        let human_data = title_map.get(&course_id).ok_or(Error::MissingField(
            "courses.courseWork.studentSubmissions{courses.courseWork[].id}",
        ))?;
        let todo = Todo {
            class_name: class_name.clone(),
            id,
            description: human_data.1.clone(),
            name: human_data.0.clone(),
            late,
        };
        todos.push(todo);
    }
    Ok(todos)
}

fn is_incomplete(sub: &StudentSubmission) -> bool {
    println!("{sub:?}");
    if is_late(sub) {
        return true;
    }
    let Some(state) = &sub.state else {
        return true;
    };
    match state.as_str() {
        "TURNED_IN" => false,
        "RETURNED" => sub.assigned_grade.is_none(),
        _ => true,
    }
}

fn is_late(sub: &StudentSubmission) -> bool {
    sub.late.map_or(false, |lateness| lateness)
}
