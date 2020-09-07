use std::sync::{Mutex, MutexGuard};

use actix_web::{App, get, HttpResponse, HttpServer, Responder, web};

const HOST: &str = "127.0.0.1";
const PORT: u32 = 8088;

// Application state - will be shared by multiple (requests processing) threads.
// Application state can be accessed with the web::Data<T> extractor where T is type of state.
// Internally, web::Data uses Arc<T>, i.e. 'Atomically Reference Counted'.
// Shared references in Rust disallow mutation by default, and Arc is no exception.
// To mutate through an Arc we need to use Mutex, RwLock, or one of the Atomic types.
struct AppState {
    teacher_name: Mutex<String>
}

// A request handler is an async function that accepts zero or more parameters that can be extracted from a request
// (ie, impl FromRequest) and returns a type that can be converted into an HttpResponse (ie, impl Responder)
async fn get_homepage() -> impl Responder {
    HttpResponse::Ok().body("This is the home page")
}

async fn get_teacher(data: web::Data<AppState>) -> impl Responder {
    let teacher_name: MutexGuard<String> = data.teacher_name.lock().unwrap(); // get MutexGuard
    // to modify
    // let mut teacher_name = data.teacher_name.lock().unwrap(); // get MutexGuard
    // *teacher_name = String::from("Another teacher"); // access teacher_name inside MutexGuard
    HttpResponse::Ok().body(format!("The teacher is: {}", teacher_name))
}

// Alternatively, you can define routes using macro attributes which allow you to specify the routes above
// your functions like so:
#[get("/students")]
async fn get_students() -> impl Responder {
    HttpResponse::Ok().body(format!("The students are: {}", "Claire David Louise"))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Initialize application state, which is shared with all routes and resources within the same scope.
    let app_state = web::Data::new(AppState {
        teacher_name: Mutex::new(String::from("Mat")),
    });

    HttpServer::new(move || {
        // Move closure transfers ownership of app_state value away from main thread
        App::new()
            // register app_state
            .app_data(app_state.clone())
            // register request handlers on a path with a method
            .route("/", web::get().to(get_homepage))
            .route("/teacher", web::get().to(get_teacher))
            // simpler registration when using macros
            .service(get_students)
    })
        .bind(format!("{}:{}", HOST, PORT))?
        .run()
        .await
}