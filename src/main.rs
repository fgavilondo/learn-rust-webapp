use std::sync::{Mutex, MutexGuard};

use actix_web::{App, Error, get, HttpRequest, HttpResponse, HttpServer, Responder, web};
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

// Alternatively, you can define routes using macro attributes which allow you to specify the routes above
// your functions like so:
#[get("/students")]
async fn get_students_html() -> impl Responder {
    HttpResponse::Ok().body(format!("The students are: {}", "Claire David Louise"))
}

// Application state - will be shared by multiple (requests processing) threads.
// Application state can be accessed with the web::Data<T> extractor where T is type of state.
// Internally, web::Data uses Arc<T>, i.e. 'Atomically Reference Counted'.
// Shared references in Rust disallow mutation by default, and Arc is no exception.
// To mutate through an Arc we need to use Mutex, RwLock, or one of the Atomic types.
struct AppState {
    teacher_name: Mutex<String>
}

async fn get_teacher_html(data: web::Data<AppState>) -> impl Responder {
    let teacher_name: MutexGuard<String> = data.teacher_name.lock().unwrap(); // get MutexGuard
    HttpResponse::Ok().body(format!("The teacher is: {}", teacher_name))
}

async fn put_teacher(data: web::Data<AppState>) -> impl Responder {
    let mut teacher_name: MutexGuard<String> = data.teacher_name.lock().unwrap();
    *teacher_name = String::from("Someone else");
    HttpResponse::Ok().body(format!("The new teacher is: {}", teacher_name))
}

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
            .route("/teacher", web::get().to(get_teacher_html))
            .route("/classroom", web::get().to(get_classroom_json))
            // simpler registration when using macros
            .service(get_students_html)
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