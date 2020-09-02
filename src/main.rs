use actix_web::{App, get, HttpResponse, HttpServer, Responder, web};

const HOST: &str = "127.0.0.1";
const PORT: u32 = 8088;

// Application state, will be shared by multiple (requests processing) threads
struct AppState {
    teacher_name: String,
}

// A request handler is an async function that accepts zero or more parameters that can be extracted from a request
// (ie, impl FromRequest) and returns a type that can be converted into an HttpResponse (ie, impl Responder)
async fn handler_homepage() -> impl Responder {
    HttpResponse::Ok().body("This is the home page")
}

// Application state can be accessed (read-only) with the web::Data<T> extractor where T is type of state.
// Internally, web::Data uses Arc<T>, i.e. 'Atomically Reference Counted'.
async fn handler_teacher(data: web::Data<AppState>) -> impl Responder {
    let teacher_name = &data.teacher_name;
    // Shared references in Rust disallow mutation by default, and Arc is no exception.
    // If you need to mutate through an Arc, use Mutex, RwLock, or one of the Atomic types.
    // data.teacher_name = String::from("Another teacher");
    HttpResponse::Ok().body(format!("The teacher is: {}", teacher_name))
}

// Alternatively, you can define routes using macro attributes which allow you to specify the routes above
// your functions like so:
#[get("/students")]
async fn handler_students() -> impl Responder {
    HttpResponse::Ok().body(format!("The students are: {}", "dipan david fabio"))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            // Initialize application state, which is shared with all routes and resources within the same scope.
            .data(AppState {
                teacher_name: String::from("Mat"),
            })
            // register request handlers on a path with a method
            .route("/", web::get().to(handler_homepage))
            .route("/teacher", web::get().to(handler_teacher))
            .service(handler_students)
    })
        .bind(format!("{}:{}", HOST, PORT))?
        .run()
        .await
}