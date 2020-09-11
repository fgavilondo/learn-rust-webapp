# learn-rust-webapp

Sample Rust actix-web webapp. Not a SPA.

git clone https://github.com/fgavilondo/learn-rust-webapp.git

# Topics covered

* actix-rt Server set-up
* actix-web App set-up
* Basic URL dispatch
* Asynchronous request handling
* GET and PUT requests
* Type-safe access to HTTP request information (actix_web::HttpRequest, web::Path, web::Json)
* JSON serialization/deserialization using the serde and serde_json crates
* Thread-safe access and modification of global application state 
* Using the Logging middleware
* Using the cookie based Session middleware
* Templates?
* ORM?

# Topics not covered

* SSL
* Authentication/Authorization
* POST and forms
* Serving static files (except for favicon.ico)
* Implementing custom middlewares
* Using application guards to filter requests, e.g. based on HTTP headers
* Testing

# Web framework: actix-web

actix-web, part of https://actix.rs/

A high level web framework built atop of the actix actor framework and the Tokio async IO system. 

It provides routing, middlewares, pre-processing of requests, post-processing of responses, etc.
 
Highly performant/concurrent.

# HTTP server

actix-rt, part of https://actix.rs/, implemented atop of the http and h2 crates.

(Other popular choices are hyper and tiny_http).

# App object

actix_web::App

Used for registering routes for resources and middlewares.

Application state is shared with all routes and resources within the same scope.
State can be accessed with the web::Data<T> extractor where T is type of state.

Access to state must be synchronised for multi-threaded modification using Mutex, RwLock, Atomic.

# Async functions

Async-await is a way to write functions that can "pause", return control to the runtime, and then pick up from where they left off.
Typically, those pauses are to wait for I/O, but there can be any number of uses.
This model is also known as "coroutines", or interleaved processing.

Implementation: async functions return a Future instead of blocking the current thread.

Future is a suspended computation that is waiting to be executed. To actually execute the future, use the .await operator.

Blocked Futures will yield control of the thread, allowing other Futures to run.

See https://rust-lang.github.io/async-book/

# Resource handlers

A request handler is a function that accepts zero or more parameters that can be extracted from a request 
(ie, impl FromRequest) and returns a type that can be converted into an HttpResponse (ie, impl Responder).

A request handler can be async, but doesn't have to.

Request handling happens in two stages. First the handler object is called, returning any object that implements 
the actix_web::Responder trait. Then, respond_to() is called on the returned object, converting itself to a HttpResponse or Error.

# DB driver

??

# ORM

?? 

# Resources

* https://www.arewewebyet.org/
* https://actix.rs/docs/installation/
* https://github.com/actix/examples
* https://qiita.com/kimagure/items/e24d7d6514a6a0dd2b48
