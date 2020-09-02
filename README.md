# learn-rust-webapp

Sample (and simple) Rust web app. Not a SPA.

# Technologies used

## Web framework

  actix-web, part of https://actix.rs/, which also includes Actor Model for concurrency

  actix-web provides various primitives to build web servers and applications with Rust. It provides routing, middlewares, pre-processing of requests, post-processing of responses, etc.

## HTTP server

  actix-rt, implemented atop of http and h2 crates

Other popular choices hyper, tiny_http

## DB driver

??

## ORM

?? 


# App object

actix_web::App

Used for registering routes for resources and middlewares.

Application state is shared with all routes and resources within the same scope. State can be accessed with the web::Data<T> extractor where T is type of state.

Must be synchronised for multi-threaded access.

# Async resource handlers

Async-await is a way to write functions that can "pause", return control to the runtime, and then pick up from where they left off. Typically those pauses are to wait for I/O, but there can be any number of uses.

This model is also known as "coroutines", or interleaved processing.

Implementation: async functions return a Future instead of blocking the current thread.

Future is a suspended computation that is waiting to be executed. To actually execute the future, use the .await operator.

Blocked Futures will yield control of the thread, allowing other Futures to run.

See https://rust-lang.github.io/async-book/


# Resources

* https://www.arewewebyet.org/
* https://actix.rs/docs/installation/
* https://github.com/actix/examples
* https://qiita.com/kimagure/items/e24d7d6514a6a0dd2b48
