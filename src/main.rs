use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use actix_files as fs;
use actix_session::{CookieSession, Session};
use actix_web::{App, get, HttpRequest, HttpResponse, HttpServer, put, Responder, Result, web};
use actix_web::middleware::Logger;
use askama::Template;
use chrono::offset::Utc;
use env_logger;
use serde::{Deserialize, Serialize};

const HOST: &str = "127.0.0.1";
const PORT: u32 = 8088;
const TEACHER_UPDATE_SESSION_PARAM: &str = "last_teacher_update";

#[derive(Clone)]
struct Student {
    id: u32,
    firstname: String,
    lastname: String,
    fav_language: String,
}

impl Student {
    fn new(id: u32, firstname: &str, lastname: &str, fav_language: &str) -> Self {
        Self {
            id,
            firstname: String::from(firstname),
            lastname: String::from(lastname),
            fav_language: String::from(fav_language),
        }
    }
}

/// Shared application state type
struct AppState {
    // Mutex (or RwLock) is necessary to mutate safely across threads
    teacher_name: Mutex<String>,
    students: Mutex<Vec<Student>>,
}

impl AppState {
    fn find_student(&self, id: u32) -> Option<Student> {
        let res: Option<Student>;
        let mutex_guard = self.students.lock().unwrap();
        for s in mutex_guard.iter() {
            if s.id == id {
                res = Some(s.clone());
                return res;
            }
        }
        None
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "welcome.html")] // using the askama template in this path, relative to the templates dir
struct WelcomeTemplate<'a> {
    // the name of the struct can be anything
    // the struct field names should match the variable names in the template
    title: &'a str,
}

/// Homepage handler
async fn get_homepage() -> Result<HttpResponse> {
    let html = WelcomeTemplate { title: "Welcome" }.render().unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate<'a> {
    title: &'a str,
}

/// 404 handler
async fn get_404_page(req: HttpRequest) -> Result<HttpResponse> {
    println!("Not Found: {:?}", req); // debugging
    let html = NotFoundTemplate { title: "Not Found" }.render().unwrap();
    Ok(HttpResponse::NotFound().content_type("text/html").body(html))
}

/// favicon handler
/// You can also define routes using macro attributes which allow you to specify the routes above
/// your functions like so:
#[get("/favicon")]
async fn get_favicon_file() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/favicon.ico")?)
}

#[derive(Template)]
#[template(path = "students.html")]
struct StudentsTemplate<'a> {
    title: &'a str,
    students: &'a [Student],
}

#[get("/students")]
async fn get_students_page(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let students = app_state.students.lock().unwrap();

    let html = StudentsTemplate {
        title: "Students",
        students: &students[..], // extract slice of all vector elements
    }.render().unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Template)]
#[template(path = "student.html")]
struct StudentTemplate<'a> {
    title: &'a str,
    firstname: &'a str,
    lastname: &'a str,
    fav_language: &'a str,
}

#[get("/students/{id}")]
async fn get_student_page(req_path: web::Path<(u32, )>, req: HttpRequest,
                          app_state: web::Data<AppState>) -> Result<HttpResponse> {
    // Use Path extractor to extract id segment from /students/{id} into tuple
    let student_id: u32 = req_path.0;
    let student_option = app_state.find_student(student_id);

    if student_option.is_none() {
        get_404_page(req).await
    } else {
        let student = student_option.unwrap();
        let html = StudentTemplate {
            title: "Student",
            firstname: &student.firstname,
            lastname: &student.lastname,
            fav_language: &student.fav_language,
        }.render().unwrap();

        Ok(HttpResponse::Ok().content_type("text/html").body(html))
    }
}

// JSON serialization using serde
#[derive(Serialize)]
struct Classroom {
    name: &'static str,
    capacity: u32,
}

#[get("/classrooms")]
async fn get_classrooms_json() -> impl Responder {
    web::Json([Classroom { name: "5VR", capacity: 25 }, Classroom { name: "2GK", capacity: 28 }])
}

#[derive(Template)]
#[template(path = "teacher.html")]
struct TeacherTemplate<'a> {
    title: &'a str,
    name: &'a str,
    last_update: &'a str,
}

fn record_teacher_update(session: &Session) {
    let result = session.set(TEACHER_UPDATE_SESSION_PARAM, Utc::now().to_rfc3339());
    result.unwrap();
}

fn get_last_teacher_update(session: &Session) -> String {
    let result = session.get::<String>(TEACHER_UPDATE_SESSION_PARAM);
    let option = result.unwrap();
    option.unwrap_or(String::from("never"))
}

#[get("/teacher")]
async fn get_teacher_page(query: web::Query<HashMap<String, String>>, session: Session,
                          app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let lock_result = app_state.teacher_name.lock();
    let mut teacher_name: MutexGuard<String> = lock_result.unwrap();

    // extractor for query parameters
    if let Some(name_query_param) = query.get("name") {
        // Form submission -> update app state.
        // This is a hack, we should really use POST for this!
        *teacher_name = name_query_param.clone();
        record_teacher_update(&session);
    }

    let html = TeacherTemplate {
        title: "Teacher",
        name: &teacher_name.to_string(),
        last_update: &get_last_teacher_update(&session),
    }.render().unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// JSON request deserialization. Must implement the Deserialize trait from serde.
#[derive(Deserialize)]
struct TeacherUpdateInfo {
    name: String,
}

/// Handler to update the teacher name stored in global application state via PUT request.
/// Teacher name specified via JSON in request body (web::Json extractor).
/// Time of update saved to session state (cookie).
#[put("/teacher")]
async fn put_teacher_via_json_req_body(json_body: web::Json<TeacherUpdateInfo>, session: Session,
                                       app_state: web::Data<AppState>) -> impl Responder {
    let lock_result = app_state.teacher_name.lock();
    let mut teacher_name: MutexGuard<String> = lock_result.unwrap();
    let previous_name: String = teacher_name.to_string().clone();
    *teacher_name = json_body.name.clone();
    record_teacher_update(&session);
    HttpResponse::Ok().body(format!("Teacher changed from '{}' to '{}'", previous_name, teacher_name))
}

/// Handler to update the teacher name stored in global application state via PUT request.
/// Teacher name specified via request path (to demonstrate using HttpRequest as an extractor).
/// Time of update saved to session state (cookie).
#[put("/teacher/{name}")]
async fn put_teacher_via_req_path(req: HttpRequest, session: Session, app_state: web::Data<AppState>) -> impl Responder {
    let lock_result = app_state.teacher_name.lock();
    let mut teacher_name: MutexGuard<String> = lock_result.unwrap();
    let previous_name: String = teacher_name.to_string().clone();
    // We can query the HttpRequest for path parameters by name:
    *teacher_name = req.match_info().get("name").unwrap().parse().unwrap();
    record_teacher_update(&session);
    HttpResponse::Ok().body(format!("Teacher changed from '{}' to '{}'", previous_name, teacher_name))
}

// This macro marks the associated async function to be executed within the actix runtime.
// We have to add actix-rt to our Cargo dependencies.
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Initialize in-memory application state. Do not use in a clustered set-up!
    let app_state = AppState {
        teacher_name: Mutex::new(String::from("Mat")),
        students: Mutex::new(vec![Student::new(1, "Claire", "Johnston", "C++"),
                                  Student::new(2, "David", "Johnston", "Java"),
                                  Student::new(3, "Lucy", "Wong", "Rust")]),
    };
    let app_state_extractor = web::Data::new(app_state);

    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let server = HttpServer::new(move || {
        // "move closure" needed to transfer ownership of app_state value from main thread
        App::new()
            // register logging middleware, it uses the standard log crate to log information.
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            // create cookie based session middleware, limited to 4000 bytes of data
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            // register app_state
            .app_data(app_state_extractor.clone())
            // register request handlers on a path with a method
            .route("/", web::get().to(get_homepage))
            // simpler registration when using macros
            .service(get_favicon_file)
            .service(get_students_page)
            .service(get_student_page)
            .service(get_classrooms_json)
            .service(get_teacher_page)
            .service(put_teacher_via_json_req_body)
            .service(put_teacher_via_req_path)
            // default
            .default_service(
                // 404 for GET request
                web::resource("").route(web::get().to(get_404_page)),
            )
    })
        // to bind ssl socket, bind_openssl() or bind_rustls() should be used.
        .bind(format!("{}:{}", HOST, PORT))?
        // HttpServer automatically starts a number of http workers, by default this number is equal to
        // the number of logical CPUs in the system
        // Once the workers are created, they each receive a separate application instance to handle requests.
        // Each worker thread processes its requests sequentially.
        .workers(8)
        .run();

    server.await
}
