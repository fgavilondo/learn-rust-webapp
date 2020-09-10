# learn-rust-webapp

Sample Rust actix-web webapp. Not a SPA.

git clone https://github.com/fgavilondo/learn-rust-webapp.git

# Topics covered

* actix-rt Server set-up
* actix-web App set-up
* Asynchronous request handling
* Basic URL dispatch
* GET and PUT requests
* Type-safe request information access (actix_web::HttpRequest, web::Path, web::Json)
* JSON serialization/deserialization using the serde and serde_json crates
* Accessing global (mutable) application state
* Logging middleware
* Cookie based session middleware
* Templates?
* ORM?

# Topics not covered

* SSL
* Authorization
* POST and forms
* Serving static files
* Implementing custom middlewares
* Using application guards to filter requests, e.g. based on HTTP headers
* Testing

# Web framework: actix-web

actix-web, part of https://actix.rs/

A high level web framework built atop of the actix actor framework and the Tokio async IO system. 

It provides routing, middlewares, pre-processing of requests, post-processing of responses, etc.
 
High performance/concurrency.

# HTTP server

actix-rt, implemented atop of the http and h2 crates.

Other popular choices are hyper and tiny_http

# App object

actix_web::App

Used for registering routes for resources and middlewares.

Application state is shared with all routes and resources within the same scope.
State can be accessed with the web::Data<T> extractor where T is type of state.

Access to state must be synchronised for multi-threaded modification using Mutex, RwLock, Atomic.

# Async resource handlers

Async-await is a way to write functions that can "pause", return control to the runtime, and then pick up from where they left off.
Typically, those pauses are to wait for I/O, but there can be any number of uses.
This model is also known as "coroutines", or interleaved processing.

Implementation: async functions return a Future instead of blocking the current thread.

Future is a suspended computation that is waiting to be executed. To actually execute the future, use the .await operator.

Blocked Futures will yield control of the thread, allowing other Futures to run.

See https://rust-lang.github.io/async-book/

# DB driver

??

# ORM

?? 

# Resources

* https://www.arewewebyet.org/
* https://actix.rs/docs/installation/
* https://github.com/actix/examples
* https://qiita.com/kimagure/items/e24d7d6514a6a0dd2b48
