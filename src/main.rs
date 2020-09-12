use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use actix_files as fs;
use actix_session::{CookieSession, Session};
use actix_web::{App, Error, get, HttpRequest, HttpResponse, HttpServer, put, Responder, Result, web};
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use askama::Template;
use chrono::offset::Utc;
use env_logger;
use futures::future::{ready, Ready};
use serde::{Deserialize, Serialize};

const HOST: &str = "127.0.0.1";
const PORT: u32 = 8088;
const TEACHER_UPDATE_SESSION_PARAM: &str = "last_teacher_update";

/// Homepage handler
async fn get_homepage() -> Result<HttpResponse> {
    let response = HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/welcome.html"));
    Ok(response)
}

/// 404 handler
async fn get_404_page(req: HttpRequest) -> Result<fs::NamedFile> {
    println!("{:?}", req); // debugging
    let file = fs::NamedFile::open("static/404.html")?.set_status_code(StatusCode::NOT_FOUND);
    Ok(file)
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
    text: &'a str,
}

async fn get_students_html() -> Result<HttpResponse> {
    let html = StudentsTemplate {
        text: "Claire, David, Louise",
    }.render().unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn get_student_html(req_path: web::Path<(u32, )>) -> impl Responder {
    // Use Path extractor to extract id segment from /students/{id} into tuple
    let student_id: u32 = req_path.0;
    match student_id {
        1 => HttpResponse::Ok().body("Claire Lisp"),
        2 => HttpResponse::Ok().body("David Haskell"),
        3 => HttpResponse::Ok().body("Louise Pascal"),
        _ => HttpResponse::NotFound().body("Unknown student ID"),
    }
}

// JSON serialization using serde
#[derive(Serialize)]
struct Classroom {
    name: &'static str,
    capacity: u32,
}

// Implement Responder trait so Classroom structs can be returned from request handlers
impl Responder for Classroom {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let json_body = serde_json::to_string(&self).unwrap();
        let response = HttpResponse::Ok().content_type("application/json").body(json_body);
        let result = Ok(response);
        // create a future that is immediately ready with a value:
        ready(result)
    }
}

async fn get_classroom_json() -> impl Responder {
    Classroom { name: "5VR", capacity: 20 }
    // Also possible, without implementing Responder:
    // web::Json(Classroom { name: "5VR", capacity: 20 })
}

#[derive(Template)]
#[template(path = "teacher.html")]
struct TeacherTemplate<'a> {
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
async fn get_teacher_html(session: Session, app_state: web::Data<AppState>,
                          query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let lock_result = app_state.teacher_name.lock();
    let mut teacher_name: MutexGuard<String> = lock_result.unwrap();

    if let Some(name_query_param) = query.get("name") {
        // Form submission -> update app state.
        // This is a hack, we should really use POST for this!
        *teacher_name = name_query_param.clone();
        record_teacher_update(&session);
    }

    let html = TeacherTemplate {
        name: &teacher_name.to_string(),
        last_update: &get_last_teacher_update(&session),
    }.render().unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Handler to update the teacher name stored in global application state via PUT request.
/// Teacher name specified via request path.
/// Time of update saved to session state (cookie).
#[put("/teacher/{name}")]
async fn put_teacher_via_req_path(session: Session, req: HttpRequest, app_state: web::Data<AppState>) -> impl Responder {
    let lock_result = app_state.teacher_name.lock();
    let mut teacher_name: MutexGuard<String> = lock_result.unwrap();
    let previous_name: String = teacher_name.to_string().clone();
    // We can query the HttpRequest for path parameters by name:
    *teacher_name = req.match_info().get("name").unwrap().parse().unwrap();
    record_teacher_update(&session);
    HttpResponse::Ok().body(format!("Teacher changed from '{}' to '{}'", previous_name, teacher_name))
}

// JSON request deserialization. Must implement the Deserialize trait from serde.
#[derive(Deserialize)]
struct TeacherUpdateInfo {
    name: String,
}

/// Handler to update the teacher name stored in global application state via PUT request.
/// Teacher name specified via JSON in request body.
/// Time of update saved to session state (cookie).
#[put("/teacher")]
async fn put_teacher_via_json_req_body(session: Session, json_body: web::Json<TeacherUpdateInfo>,
                                       app_state: web::Data<AppState>) -> impl Responder {
    let lock_result = app_state.teacher_name.lock();
    let mut teacher_name: MutexGuard<String> = lock_result.unwrap();
    let previous_name: String = teacher_name.to_string().clone();
    *teacher_name = json_body.name.clone();
    record_teacher_update(&session);
    HttpResponse::Ok().body(format!("Teacher changed from '{}' to '{}'", previous_name, teacher_name))
}

/// Shared application state type
struct AppState {
    // Mutex (or RwLock) is necessary to mutate safely across threads
    teacher_name: Mutex<String>
}

// This macro marks the associated async function to be executed within the actix runtime.
// We have to add actix-rt to our Cargo dependencies.
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Initialize application state. Do not use in a clustered set-up!
    let app_state = AppState {
        teacher_name: Mutex::new(String::from("Mat")),
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
            .route("/students", web::get().to(get_students_html))
            .route("/students/{id}", web::get().to(get_student_html))
            .route("/classroom", web::get().to(get_classroom_json))
            // simpler registration when using macros
            .service(get_teacher_html)
            .service(put_teacher_via_req_path)
            .service(put_teacher_via_json_req_body)
            .service(get_favicon_file)
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
