# learn-rust-webapp

Sample Rust actix-web webapp. Server-side rendered HTML, not a SPA.

    git clone https://github.com/fgavilondo/learn-rust-webapp.git

# Topics covered

* actix-web App and HTTP Server set-up
* Basic URL dispatch
* (Asynchronous) Request handling
* GET and PUT requests
* Thread-safe access to global application state 
* Type-safe access to HTTP Request information
* JSON serialization/deserialization using the serde and serde_json crates
* Using middlewares (Logging, cookie based Session)
* Askama templating engine
* ORM?

# Topics not covered

* SSL/TLS
* Authentication/Authorization
* POST and HTML forms
* Serving static files (except for favicon.ico)
* Implementing custom middlewares
* Using application guards to filter requests, e.g. based on HTTP headers
* Unit/integration testing

# Web framework

actix-web, part of https://actix.rs/

A high level web framework providing routing, middlewares, pre-processing of requests, post-processing of responses, etc.

Built atop of the actix actor framework and the Tokio async IO system -> highly performant/concurrent.

(Other popular web frameworks are rocket and gotham).

# Detour: Coroutines

Async/Await is a way to write functions that can "pause", return control to the runtime, and then pick up from where they left off.
Typically, those pauses are to wait for I/O, but there can be any number of uses.

This model is also known as "coroutines", or interleaved processing. It is an example of non-preemptive multitasking.
It allows writing asynchronous, non-blocking code with minimal overhead, and looking almost like traditional synchronous, blocking code. 

async functions return a Future object that can be used to block and wait for the operation to complete at some other convenient time.
 
Example:

    use futures::executor::block_on;
    
    async fn print_one() {
        print!(" 1 ");
    }
    
    async fn print_one_two() {
        // Inside an async fn, you can use .await to wait for the completion of another type that implements
        // the Future trait, such as the output of another async fn.
        // Unlike block_on(), .await doesn't block the current thread, but instead asynchronously waits for
        // the future to complete. In the meantime, other async functions can run.
        print_one().await;
        print!(" 2 ");
    }
    
    async fn print_three() {
        print!(" 3 ");
    }
    
    async fn print_one_two_three_maybe() {
        let f1 = print_one_two();  // nothing printed, returns a future
        let f2 = print_three();    // nothing printed, returns a future
    
        // `join!` is like `.await` but can wait for multiple futures concurrently.
        // If we're temporarily blocked in one future, another
        // future will take over the current thread. If both futures are blocked, then
        // this function is blocked and will yield to the executor.
        futures::join!(f1, f2);
    }
    
    fn main() {
        // blocks the current thread until the provided future has run to completion.
        block_on(print_one_two_three_maybe());
    }

Most actix-web request handlers are implemented as async functions.

See https://rust-lang.github.io/async-book/ for more information.

Languages with async/await syntax: Rust, C#, Kotlin, JavaScript, Python
Notable exceptions: Java, Go (goroutines)

# actix_web::HttpServer

Responsible for serving HTTP requests. Accepts an application factory as a parameter.

Use bind() method to bind to a specific socket address.
To bind SSL socket, use bind_openssl() or bind_rustls(). 

HttpServer automatically starts a number of HTTP workers, by default this number is equal to the number of logical
CPUs in the system. This number can be overridden with the workers() method.

The run() Method returns an instance of the Server type which can be .await(ed)

Server methods:

pause() - Pause accepting incoming connections
resume() - Resume accepting incoming connections
stop() - Stop incoming connection processing, stop all workers and exit

Other popular HTTP server choices are hyper and tiny_http.

# actix_web::App object

Used for URL dispatch, i.e. registering routes for resources and middlewares.

Application state is shared with all routes and resources within the same scope.
Application state can be accessed with the web::Data<T> extractor where T is type of state.

Access to shared app state must be synchronised for multi-threaded modification using Mutex, RwLock or Atomic.

# Request handlers

A request handler is a function that accepts zero or more parameters that can be extracted from a request 
(ie, impl actix_web::FromRequest trait) and returns either a HttpResponse directly, or a type that can be converted into 
a HttpResponse (ie, impl actix_web::Responder trait).

Request handling happens in two stages. First the handler object is called, returning any object that implements 
the Responder trait. Then, respond_to() is called on the returned object, converting itself to a HttpResponse or Error.

By default actix-web provides Responder implementations for some standard types, such as &'static str and String, as
well as for actix-web types such as NamedFile.

To return your custom type directly from a handler function, the type needs to implement the Responder trait.

Any long, non-cpu-bound operation (e.g. I/O, database operations, etc.) should be expressed as futures or
asynchronous functions.

Request handlers are registered with the App using the route() or service() method.

# Type-safe access to HTTP Request information 

Actix-web provides a facility for type-safe request information access called extractors (ie, impl FromRequest).
You can define an extractor as a parameter of your request handler.

By default, actix-web provides several extractor implementations, e.g.: 

* actix_web::HttpRequest - HttpRequest itself is an extractor which returns self, in case you need access to the request.
* web::Path - Path provides information that can be extracted from the Request’s path. You can deserialize any variable segment from the path, e.g. by extracting the segments into a tuple.
* web::Json - Allows deserialization of a request body into a struct. To extract typed information from a request’s body, the type T must implement the Deserialize trait from serde.
* web::Query - Provides extraction functionality for the request’s query parameters. Underneath it uses serde_urlencoded crate.

Others (not used in this app): Form, String, ...

# JSON Serialization/Deserialization

* https://crates.io/crates/serde
* https://crates.io/crates/serde_json

## Serialization

Serde provides a 'derive' macro to generate a simple, 1:1 serialization implementation for structs in your own program:

    #[derive(Serialize)]
    struct MyStruct {
      // ...
    }

Alternatively, you can provide your own custom implementation of the Serialize trait:

    impl Serialize for MyStruct {
        fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
            S: Serializer {
            // ... here you could do all sorts of fancy stuff, e.g. combine fields 
        }
    }

Actually serialize using:

    serde_json::to_string(&MyStruct).unwrap();


## Deserialization

Use the provided 'derive' macro to make your structs deserializable:

    #[derive(Deserialize)]
    struct MyStruct {
        // ...
    }

... and use the web::Json extractor to deserialize them from the HTTP request.

# In-memory application state

Application state (usually a struct) is registered with the App when server is initialised. It can be accessed in your request handlers with the web::Data<T> extractor where T is type of state.

Application state is shared by multiple (requests processing) threads. Internally, web::Data uses Arc<T>, i.e. 'Atomically Reference Counted'.

Shared references in Rust disallow mutation by default, and Arc is no exception. To mutate through an Arc we need to use Mutex, RwLock, or one of the Atomic types.

# Session state

The actix-session middleware can be used with different backend types to store session data in different backends.

By default, only cookie session backend is implemented. Other backend implementations can be added.

A cookie may have a security policy of signed or private. Each has a respective CookieSession constructor.
A signed cookie may be viewed but not modified by the client. A private cookie may neither be viewed nor modified by the client.

To access session data the actix_session::Session extractor must be used. 

# Template Rendering

Multiple options:

* [Askama](https://crates.io/crates/askama): a template rendering engine based on [Jinja](https://palletsprojects.com/p/jinja/).
It generates Rust code from your templates at compile time based on a user-defined struct to hold the template's context.
* [Handlebars](https://crates.io/crates/handlebars): [Handlebars templating language](https://handlebarsjs.com/) implemented in Rust.
* [Tera](https://crates.io/crates/tera): a template engine inspired by Jinja2 and the Django template language.
* [Yarte](https://crates.io/crates/yarte): Yarte stands for Yet Another Rust Template Engine. It uses a Handlebars-like syntax.
* [TinyTemplate](https://crates.io/crates/tinytemplate): a small, minimalistic text templating system with limited dependencies.

I picked Askama because I've used Jinja with Python before. Fast (compiled). Drawback: must restart app if HTML changes.

Askama features:

* Template inheritance
* Loops, if/else statements and include support
* Macro support
* Variables (no mutability allowed)
* Some built-in filters, and the ability to use your own
* Whitespace suppressing with '-' markers
* Opt-out HTML escaping
* Syntax customization

# DB driver

??

# ORM

?? 

# Resources

* https://www.arewewebyet.org/
* https://actix.rs/docs/installation/
* https://github.com/actix/examples
