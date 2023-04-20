use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::Html,
};
use classroom::{
    api::{Course, CourseWork, StudentSubmission, TimeOfDay},
    chrono::{DateTime, NaiveDate, NaiveTime, Utc},
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
    assignment_list.sort_by(|a, b| a.due.cmp(&b.due).reverse());
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
    class_id: String,
    id: String,
    description: Option<String>,
    name: Option<String>,
    late: bool,
    due: Option<DueDateTime>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct DueDateTime(DateTime<Utc>);

impl std::ops::Deref for DueDateTime {
    type Target = DateTime<Utc>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for DueDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.timestamp_millis())
    }
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
        .param("fields", "courseWork(id,title,description,dueDate,dueTime)")
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
    let mut course_works_by_id: HashMap<String, CourseWork> = HashMap::new();
    for course in course_works {
        if let Some(id) = course.id.clone() {
            course_works_by_id.insert(id, course);
        }
    }
    let mut todos = Vec::new();
    let submissions: Vec<StudentSubmission> =
        submissions.into_iter().filter(is_incomplete).collect();
    for submission in submissions {
        let late = is_late(&submission);
        let work_id = submission.course_work_id.ok_or(Error::MissingField(
            "courses.courseWork.studentSubmissions[].courseWorkId",
        ))?;
        let id = submission.id.ok_or(Error::MissingField(
            "courses.courseWork.studentSubmissions[].id",
        ))?;
        let course = course_works_by_id.get(&work_id).ok_or(Error::MissingField(
            "courses.courseWork.studentSubmissions{courses.courseWork[].id}",
        ))?;
        let due = course.due_date.as_ref().and_then(|due_date| {
            let due_time = course.due_time.clone().unwrap_or(TimeOfDay {
                hours: Some(0),
                minutes: Some(0),
                seconds: Some(0),
                nanos: Some(0),
            });
            classroom_to_naivedate(due_date, &due_time)
        });
        let todo = Todo {
            class_name: class_name.clone(),
            class_id: course_id.clone(),
            id,
            description: course.description.clone(),
            name: course.title.clone(),
            late,
            due,
        };
        todos.push(todo);
    }
    todos.sort_by(|a, b| a.due.cmp(&b.due).reverse());
    Ok(todos)
}

fn is_incomplete(sub: &StudentSubmission) -> bool {
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

fn classroom_to_naivedate(
    classroom_date: &classroom::api::Date,
    classroom_time: &classroom::api::TimeOfDay,
) -> Option<DueDateTime> {
    let date = NaiveDate::from_ymd_opt(
        classroom_date.year?,
        classroom_date.month?.try_into().ok()?,
        classroom_date.day?.try_into().ok()?,
    )?;
    let time = NaiveTime::from_hms_nano_opt(
        classroom_time.hours.unwrap_or(0).try_into().ok()?,
        classroom_time.minutes.unwrap_or(0).try_into().ok()?,
        classroom_time.seconds.unwrap_or(0).try_into().ok()?,
        classroom_time.nanos.unwrap_or(0).try_into().ok()?,
    )?;
    Some(DueDateTime(classroom::chrono::DateTime::<Utc>::from_utc(
        classroom::chrono::NaiveDateTime::new(date, time),
        Utc,
    )))
}
