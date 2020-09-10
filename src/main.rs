use std::sync::{Mutex, MutexGuard};

use actix_web::{App, Error, get, HttpRequest, HttpResponse, HttpServer, put, Responder, web};
use futures::future::{ready, Ready};
use serde::Serialize;

const HOST: &str = "127.0.0.1";
const PORT: u32 = 8088;

// A request handler is a function that accepts zero or more parameters that can be extracted from a request
// (ie, impl FromRequest) and returns a type that can be converted into an HttpResponse (ie, impl Responder)
// Any long, non-cpu-bound operation (e.g. I/O, database operations, etc.) should be expressed as futures or
// asynchronous functions. Async handlers get executed concurrently by worker threads and thus don’t block execution.
async fn get_homepage_html() -> impl Responder {
    HttpResponse::Ok().body("This is the home page")
}

async fn get_students_html() -> impl Responder {
    HttpResponse::Ok().body(format!("The students are: Claire, David, Louise"))
}

// Path provides information that can be extracted from the Request’s path.
// You can deserialize any variable segment from the path, e.g. by extracting the segments into a tuple.
async fn get_student_html(path: web::Path<(u32, )>) -> impl Responder {
    // extract path info from /students/{id}
    let student_id: u32 = path.0;
    match student_id {
        1 => HttpResponse::Ok().body("Claire Lisp"),
        2 => HttpResponse::Ok().body("David Haskell"),
        3 => HttpResponse::Ok().body("Louise Pascal"),
        _ => HttpResponse::NotFound().body("Unknown student ID"),
    }
}

// JSON serialization
#[derive(Serialize)]
struct Classroom {
    name: &'static str,
    capacity: u32,
}

// Types that implement Responder can be used as the return type of a request handler
impl Responder for Classroom {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();
        ready(Ok(HttpResponse::Ok().content_type("application/json").body(body)))
    }
}

async fn get_classroom_json() -> impl Responder {
    Classroom { name: "5VR", capacity: 20 }
}

// Application state - will be shared by multiple (requests processing) threads.
// Application state can be accessed with the web::Data<T> extractor where T is type of state.
// Internally, web::Data uses Arc<T>, i.e. 'Atomically Reference Counted'.
// Shared references in Rust disallow mutation by default, and Arc is no exception.
// To mutate through an Arc we need to use Mutex, RwLock, or one of the Atomic types.
struct AppState {
    teacher_name: Mutex<String>
}

// You can also define routes using macro attributes which allow you to specify the routes above
// your functions like so:
#[get("/teacher")]
async fn get_teacher_html(data: web::Data<AppState>) -> impl Responder {
    let teacher_name: MutexGuard<String> = data.teacher_name.lock().unwrap(); // get MutexGuard
    HttpResponse::Ok().body(format!("The teacher is: {}", teacher_name))
}

#[put("/teacher/{name}")]
async fn put_teacher(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    // Instead of using Path, it is also possible to get or query the request for path parameters by name:
    let new_name: String =
        req.match_info().get("name").unwrap().parse().unwrap();
    let mut teacher_name: MutexGuard<String> = data.teacher_name.lock().unwrap();
    *teacher_name = new_name;
    HttpResponse::Ok().body(format!("Teacher changed to: {}", teacher_name))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // initialize application state, shared with all routes and resources within the same scope.
    let app_state = web::Data::new(AppState {
        teacher_name: Mutex::new(String::from("Mat")),
    });

    HttpServer::new(move || {
        // "move closure" transfers ownership of app_state value away from main thread
        App::new()
            // register app_state
            .app_data(app_state.clone())
            // register request handlers on a path with a method
            .route("/", web::get().to(get_homepage_html))
            .route("/students", web::get().to(get_students_html))
            .route("/students/{id}", web::get().to(get_student_html))
            .route("/classroom", web::get().to(get_classroom_json))
            // simpler registration when using macros
            .service(get_teacher_html)
            .service(put_teacher)
    })
        // to bind ssl socket, bind_openssl() or bind_rustls() should be used.
        .bind(format!("{}:{}", HOST, PORT))?
        // HttpServer automatically starts a number of http workers, by default this number is equal to
        // the number of logical CPUs in the system
        // Once the workers are created, they each receive a separate application instance to handle requests.
        // Each worker thread processes its requests sequentially.
        .workers(8)
        .run()
        .await
}