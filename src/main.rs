use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use actix_files::NamedFile;
use actix_session::{CookieSession, Session};
use actix_web::{App, get, HttpRequest, HttpResponse, HttpServer, post, put, Result, web};
use actix_web::middleware::Logger;
use askama::Template;
use chrono::offset::Utc;
use env_logger;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use rustls::{NoClientAuth, ServerConfig};
use rustls::internal::pemfile::{certs, rsa_private_keys};
use serde::{Deserialize, Serialize};

const HOST: &str = "127.0.0.1";
// const PORT: u32 = 8088;
// use different port for HTTPS
const PORT: u32 = 8443;
const LAST_STUDENT_POST_SESSION_PARAM: &str = "last_student_post";

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

/// Askama template data for homepage
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

/// Askama template data for 404 page
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
async fn get_favicon_file() -> Result<NamedFile> {
    Ok(NamedFile::open("static/favicon.ico")?)
}

fn get_last_student_post_time(session: &Session) -> String {
    session.get::<String>(LAST_STUDENT_POST_SESSION_PARAM).unwrap().unwrap_or(String::from("never"))
}

fn record_student_post_time(session: &Session) {
    session.set(LAST_STUDENT_POST_SESSION_PARAM, Utc::now().to_rfc3339()).unwrap();
}

/// Askama template data for Students page
#[derive(Template)]
#[template(path = "students.html")]
struct StudentsTemplate<'a> {
    title: &'a str,
    students: &'a [Student],
    last_post: &'a str,
}

#[get("/students")]
async fn get_students_page(session: Session, app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let students = app_state.students.lock().unwrap();

    let html = StudentsTemplate {
        title: "Students",
        students: &students[..], // extract slice of all vector elements
        last_post: &get_last_student_post_time(&session),
    }.render().unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Must implement the Deserialize trait from serde for url-encoded form deserialization.
#[derive(Deserialize)]
struct NewStudentFormData {
    fname: String,
    lname: String,
    lang: String,
}

/// Handler to create a new student resource under /students via POST request.
/// Gets called only if the content type is "application/x-www-form-urlencoded".
/// and the content of the request could be deserialized to a `TeacherUpdateInfo` struct.
/// Timestamp of last POST saved to session state (cookie).
#[post("/students")]
async fn post_student(form: web::Form<NewStudentFormData>, session: Session,
                      app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let mut students = app_state.students.lock().unwrap();
    let new_student =
        Student::new(students.len() as u32 + 1, &form.fname, &form.lname, &form.lang);
    students.push(new_student);

    record_student_post_time(&session);

    let html = StudentsTemplate {
        title: "Students",
        students: &students[..], // extract slice of all vector elements
        last_post: &get_last_student_post_time(&session),
    }.render().unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Askama template data for Student page
#[derive(Template)]
#[template(path = "student.html")]
struct StudentTemplate<'a> {
    title: &'a str,
    firstname: &'a str,
    lastname: &'a str,
    fav_language: &'a str,
}

/// Use Path extractor to extract id segment from /students/{id} into tuple
#[get("/students/{id}")]
async fn get_student_page(web::Path((student_id, )): web::Path<(u32, )>,
                          req: HttpRequest,
                          app_state: web::Data<AppState>) -> Result<HttpResponse> {
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
    name: String,
    capacity: u32,
}

fn db_read_classrooms(db: web::Data<Pool<SqliteConnectionManager>>) -> Vec<Classroom> {
    let conn = db.get().unwrap();

    let mut stmt = conn.prepare("SELECT name, capacity FROM classroom").expect("Database connection error");
    let query_result = stmt.query_map(params![], |row| {
        Ok(Classroom {
            name: row.get(0)?,
            capacity: row.get(1)?,
        })
    });
    let rows = query_result.unwrap();

    let mut classrooms: Vec<Classroom> = Vec::new();
    for classroom in rows {
        classrooms.push(classroom.unwrap());
    }

    classrooms
}

#[get("/classrooms")]
async fn get_classrooms_json(db: web::Data<Pool<SqliteConnectionManager>>) -> Result<HttpResponse> {
    let classrooms: Vec<Classroom> = db_read_classrooms(db);
    Ok(HttpResponse::Ok().json(classrooms))
}

/// Askama template data for Teacher page
#[derive(Template)]
#[template(path = "teacher.html")]
struct TeacherTemplate<'a> {
    title: &'a str,
    name: &'a str,
}

#[get("/teacher")]
async fn get_teacher_page(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let teacher_name: MutexGuard<String> = app_state.teacher_name.lock().unwrap();

    let html = TeacherTemplate {
        title: "Teacher",
        name: &teacher_name.to_string(),
    }.render().unwrap();

    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Must implement the Deserialize trait from serde for JSON deserialization.
#[derive(Deserialize)]
struct TeacherUpdateInfo {
    name: String,
}

/// Handler to update the teacher name stored in global application state via PUT request.
/// Teacher name specified via JSON in request body (web::Json extractor).
#[put("/teacher")]
async fn put_teacher_via_json_req_body(json_body: web::Json<TeacherUpdateInfo>,
                                       app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let mut teacher_name: MutexGuard<String> = app_state.teacher_name.lock().unwrap();
    let previous_name: String = teacher_name.to_string().clone();
    *teacher_name = json_body.name.clone();
    let resp_body: String = format!("Teacher changed from '{}' to '{}'", previous_name, teacher_name);
    Ok(HttpResponse::Ok().content_type("text/plain").body(resp_body))
}

/// Handler for serving named files out of the /static directory
#[get("/static/{filename:.*}")]
async fn serve_static_file(req: HttpRequest) -> Result<NamedFile> {
    let filename: std::path::PathBuf = req.match_info().query("filename").parse().unwrap();
    let mut path = PathBuf::from("static/");
    path.push(filename);
    let file = NamedFile::open(path)?;
    Ok(file.use_etag(true).use_last_modified(true))
}

fn build_ssl_server_config() -> ServerConfig {
    let mut server_config = ServerConfig::new(NoClientAuth::new());
    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());
    let cert_chain = certs(cert_file).unwrap();
    let mut keys = rsa_private_keys(key_file).unwrap();
    server_config.set_single_cert(cert_chain, keys.remove(0)).unwrap();
    server_config
}

fn db_create_schema(db: &Pool<SqliteConnectionManager>) {
    let conn = db.get().unwrap();
    conn.execute(
        "CREATE TABLE classroom (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL,
                  capacity        INTEGER
                  )",
        params![],
    ).expect("Database connection error");
}

fn db_insert_classroom(db: &Pool<SqliteConnectionManager>, name: &str, capacity: u32) {
    let conn = db.get().unwrap();
    conn.execute(
        "INSERT INTO classroom (name, capacity) VALUES (?1, ?2)",
        params![name, capacity],
    ).expect("Database connection error");
}

fn init_database() -> Pool<SqliteConnectionManager> {
    // let db_conn_manager: SqliteConnectionManager = SqliteConnectionManager::file("school.db");
    // use in-memory DB for simplicity
    let db_conn_manager: SqliteConnectionManager = SqliteConnectionManager::memory();
    let db_conn_pool: Pool<SqliteConnectionManager> = r2d2::Pool::new(db_conn_manager).unwrap();

    // since we're using an in-memory DB, we have to seed it with some values
    db_create_schema(&db_conn_pool);
    db_insert_classroom(&db_conn_pool, "5VR", 35);
    db_insert_classroom(&db_conn_pool, "2GK", 38);

    db_conn_pool
}

// This macro marks the associated async function to be executed within the actix runtime.
// We have to add actix-rt to our Cargo dependencies.
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();

    // Initialize in-memory application state. Do not use in a clustered set-up!
    let app_state = AppState {
        teacher_name: Mutex::new(String::from("Louise")),
        students: Mutex::new(vec![Student::new(1, "Claire", "Johnston", "C++"),
                                  Student::new(2, "David", "Johnston", "Java"),
                                  Student::new(3, "Mark", "Wong", "Rust")]),
    };

    let app_state_extractor = web::Data::new(app_state);
    let db_conn_pool: Pool<SqliteConnectionManager> = init_database();
    let server_config = build_ssl_server_config();

    let server = HttpServer::new(move || {
        // "move closure" needed to transfer ownership of values from main thread
        App::new()
            // create cookie based session middleware, limited to 4000 bytes of data
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            // enable logger - always register actix-web Logger middleware last
            .wrap(Logger::default())
            // register app_state
            .app_data(app_state_extractor.clone())
            .data(db_conn_pool.clone())
            // register request handlers on a path with a method
            .route("/", web::get().to(get_homepage))
            // simpler registration when using macros
            .service(get_favicon_file)
            .service(get_students_page)
            .service(get_student_page)
            .service(post_student)
            .service(get_classrooms_json)
            .service(get_teacher_page)
            .service(put_teacher_via_json_req_body)
            .service(serve_static_file)
            // default
            .default_service(
                // 404 for GET request
                web::resource("").route(web::get().to(get_404_page)),
            )
    })
        // bind plain socket
        // .bind(format!("{}:{}", HOST, PORT))?
        // to bind ssl socket, bind_openssl() or bind_rustls() should be used.
        .bind_rustls(format!("{}:{}", HOST, PORT), server_config)?
        // HttpServer automatically starts a number of http workers, by default this number is equal to
        // the number of logical CPUs in the system.
        // Once the workers are created, they each receive a separate application instance to handle requests.
        // Each worker thread processes its requests sequentially.
        .workers(4)
        .run();

    server.await
}

#[cfg(test)]
mod tests {
    use actix_http::{Request, Response};
    use actix_web::{http, test};
    use actix_web::body::Body::Bytes;

    use super::*;

    // Unit tests (test individual request handler functions)
    #[actix_rt::test]
    async fn unit_test_homepage_contents() {
        let resp: Response = get_homepage().await.unwrap();
        assert_eq!(resp.status(), http::StatusCode::OK);

        let body: String = get_response_body(&resp);
        assert!(body.contains("Welcome!"));
        assert!(body.contains("students"));
        assert!(body.contains("teacher"));
        assert!(body.contains("classrooms"));
    }

    fn get_response_body(resp: &Response) -> String {
        let response_body = match resp.body().as_ref() {
            Some(Bytes(bytes)) => bytes,
            _ => panic!("Response error"),
        };
        String::from_utf8(response_body.to_vec()).expect("Invalid UTF-8")
    }

    #[actix_rt::test]
    async fn unit_test_404() {
        let req: HttpRequest = test::TestRequest::with_uri("/wrongpage").to_http_request();
        let resp: Response = get_404_page(req).await.unwrap();
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);

        let body: String = get_response_body(&resp);
        assert!(body.contains("Back to home"));
    }

    // Integration tests (run the application with specific request handlers in a real HTTP server)

    #[actix_rt::test]
    async fn integration_can_get_homepage() {
        let mut app =
            test::init_service(App::new().route("/", web::get().to(get_homepage))).await;
        let req: Request = test::TestRequest::with_header("content-type", "text/html").to_request();

        let service_resp = test::call_service(&mut app, req).await;
        assert!(service_resp.status().is_success());

        let resp: &Response = service_resp.response();
        assert_eq!(resp.status(), http::StatusCode::OK);

        let body: String = get_response_body(resp);
        assert!(body.contains("Welcome!"));
    }

    #[actix_rt::test]
    async fn integration_cannot_post_homepage() {
        let mut app =
            test::init_service(App::new().route("/", web::get().to(get_homepage))).await;
        let req: Request = test::TestRequest::post().uri("/").to_request();

        let service_resp = test::call_service(&mut app, req).await;
        assert!(service_resp.status().is_client_error());
    }
}
