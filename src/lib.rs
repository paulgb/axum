//! axum is a web application framework that focuses on ergonomics and modularity.
//!
//! # Table of contents
//!
//! - [High level features](#high-level-features)
//! - [Compatibility](#compatibility)
//! - [Handlers](#handlers)
//!     - [Debugging handler type errors](#debugging-handler-type-errors)
//! - [Routing](#routing)
//!     - [Wildcard routes](#wildcard-routes)
//!     - [Nesting routes](#nesting-routes)
//!     - [Fallback routes](#fallback-routes)
//!     - [Routing to any `Service`](#routing-to-any-service)
//!         - [Routing to fallible services](#routing-to-fallible-services)
//! - [Extractors](#extractors)
//!     - [Common extractors](#common-extractors)
//!     - [Applying multiple extractors](#applying-multiple-extractors)
//!     - [Optional extractors](#optional-extractors)
//!     - [Customizing extractor responses](#customizing-extractor-responses)
//! - [Building responses](#building-responses)
//! - [Error handling](#error-handling)
//! - [Applying middleware](#applying-middleware)
//!     - [To individual handlers](#to-individual-handlers)
//!     - [To groups of routes](#to-groups-of-routes)
//!     - [Applying multiple middleware](#applying-multiple-middleware)
//!     - [Commonly used middleware](#commonly-used-middleware)
//!     - [Writing your own middleware](#writing-your-own-middleware)
//! - [Sharing state with handlers](#sharing-state-with-handlers)
//! - [Required dependencies](#required-dependencies)
//! - [Examples](#examples)
//! - [Feature flags](#feature-flags)
//!
//! # High level features
//!
//! - Route requests to handlers with a macro free API.
//! - Declaratively parse requests using extractors.
//! - Simple and predictable error handling model.
//! - Generate responses with minimal boilerplate.
//! - Take full advantage of the [`tower`] and [`tower-http`] ecosystem of
//!   middleware, services, and utilities.
//!
//! In particular the last point is what sets `axum` apart from other frameworks.
//! `axum` doesn't have its own middleware system but instead uses
//! [`tower::Service`]. This means `axum` gets timeouts, tracing, compression,
//! authorization, and more, for free. It also enables you to share middleware with
//! applications written using [`hyper`] or [`tonic`].
//!
//! # Compatibility
//!
//! axum is designed to work with [tokio] and [hyper]. Runtime and
//! transport layer independence is not a goal, at least for the time being.
//!
//! # Example
//!
//! The "Hello, World!" of axum is:
//!
//! ```rust,no_run
//! use axum::{
//!     routing::get,
//!     Router,
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     // build our application with a single route
//!     let app = Router::new().route("/", get(|| async { "Hello, World!" }));
//!
//!     // run it with hyper on localhost:3000
//!     axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
//!         .serve(app.into_make_service())
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! # Handlers
//!
//! In axum a "handler" is an async function that accepts zero or more
//! ["extractors"](#extractors) as arguments and returns something that
//! can be converted [into a response](#building-responses).
//!
//! Handlers is where your custom domain logic lives and axum applications are
//! built by routing between handlers.
//!
//! Some examples of handlers:
//!
//! ```rust
//! use bytes::Bytes;
//! use http::StatusCode;
//!
//! // Handler that immediately returns an empty `200 OK` response.
//! async fn unit_handler() {}
//!
//! // Handler that immediately returns an empty `200 OK` response with a plain
//! // text body.
//! async fn string_handler() -> String {
//!     "Hello, World!".to_string()
//! }
//!
//! // Handler that buffers the request body and returns it.
//! async fn echo(body: Bytes) -> Result<String, StatusCode> {
//!     if let Ok(string) = String::from_utf8(body.to_vec()) {
//!         Ok(string)
//!     } else {
//!         Err(StatusCode::BAD_REQUEST)
//!     }
//! }
//! ```
//!
//! ## Debugging handler type errors
//!
//! For a function to used as a handler it must implement the [`Handler`] trait.
//! axum provides blanket implementations for functions that:
//!
//! - Are `async fn`s.
//! - Take no more than 16 arguments that all implement [`FromRequest`].
//! - Returns something that implements [`IntoResponse`].
//! - If a closure is used it must implement `Clone + Send + Sync` and be
//! `'static`.
//! - Returns a future that is `Send`. The most common way to accidentally make a
//! future `!Send` is to hold a `!Send` type across an await.
//!
//! Unfortunately Rust gives poor error messages if you try to use a function
//! that doesn't quite match what's required by [`Handler`].
//!
//! You might get an error like this:
//!
//! ```not_rust
//! error[E0277]: the trait bound `fn(bool) -> impl Future {handler}: Handler<_, _>` is not satisfied
//!    --> src/main.rs:13:44
//!     |
//! 13  |     let app = Router::new().route("/", get(handler));
//!     |                                            ^^^^^^^ the trait `Handler<_, _>` is not implemented for `fn(bool) -> impl Future {handler}`
//!     |
//!    ::: axum/src/handler/mod.rs:116:8
//!     |
//! 116 |     H: Handler<B, T>,
//!     |        ------------- required by this bound in `axum::routing::get`
//! ```
//!
//! This error doesn't tell you _why_ your function doesn't implement
//! [`Handler`]. It's possible to improve the error with the [`debug_handler`]
//! proc-macro from the [axum-debug] crate.
//!
//! # Routing
//!
//! [`Router::route`] is the main way to add routes:
//!
//! ```rust,no_run
//! use axum::{
//!     routing::get,
//!     Router,
//! };
//!
//! let app = Router::new()
//!     .route("/", get(get_slash).post(post_slash))
//!     .route("/foo", get(get_foo));
//!
//! async fn get_slash() {
//!     // `GET /` called
//! }
//!
//! async fn post_slash() {
//!     // `POST /` called
//! }
//!
//! async fn get_foo() {
//!     // `GET /foo` called
//! }
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Routes can also be dynamic like `/users/:id`. See [extractors](#extractors)
//! for more details.
//!
//! You can also define routes separately and merge them with [`Router::merge`].
//!
//! Routes are not allowed to overlap and will panic if an overlapping route is
//! added. This also means the order in which routes are added doesn't matter.
//!
//! ## Wildcard routes
//!
//! axum also supports wildcard routes:
//!
//! ```rust,no_run
//! use axum::{
//!     routing::get,
//!     Router,
//! };
//!
//! let app = Router::new()
//!     // this matches any request that starts with `/api`
//!     .route("/api/*rest", get(|| async { /* ... */ }));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! The matched path can be extracted via [`extract::Path`]:
//!
//! ```rust,no_run
//! use axum::{
//!     routing::get,
//!     extract::Path,
//!     Router,
//! };
//!
//! let app = Router::new().route("/api/*rest", get(|Path(rest): Path<String>| async {
//!     // `rest` will be everything after `/api`
//! }));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! ## Nesting routes
//!
//! Routes can be nested by calling [`Router::nest`](routing::Router::nest):
//!
//! ```rust,no_run
//! use axum::{
//!     body::{Body, BoxBody},
//!     http::Request,
//!     routing::get,
//!     Router,
//! };
//! use tower_http::services::ServeFile;
//! use http::Response;
//!
//! fn api_routes() -> Router {
//!     Router::new()
//!         .route("/users", get(|_: Request<Body>| async { /* ... */ }))
//! }
//!
//! let app = Router::new()
//!     .route("/", get(|_: Request<Body>| async { /* ... */ }))
//!     .nest("/api", api_routes());
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Note that nested routes will not see the orignal request URI but instead
//! have the matched prefix stripped. This is necessary for services like static
//! file serving to work. Use [`OriginalUri`] if you need the original request
//! URI.
//!
//! Nested routes are similar to wild card routes. The difference is that
//! wildcard routes still see the whole URI whereas nested routes will have
//! the prefix stripped.
//!
//! ```rust
//! use axum::{routing::get, http::Uri, Router};
//!
//! let app = Router::new()
//!     .route("/foo/*rest", get(|uri: Uri| async {
//!         // `uri` will contain `/foo`
//!     }))
//!     .nest("/bar", get(|uri: Uri| async {
//!         // `uri` will _not_ contain `/bar`
//!     }));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! ## Fallback routes
//!
//! By default axum will respond with an empty `404 Not Found` response to unhandled requests. To
//! override that you can use [`Router::fallback`]:
//!
//! ```rust
//! use axum::{
//!     Router,
//!     routing::get,
//!     handler::Handler,
//!     response::IntoResponse,
//!     http::{StatusCode, Uri},
//! };
//!
//! async fn fallback(uri: Uri) -> impl IntoResponse {
//!     (StatusCode::NOT_FOUND, format!("No route for {}", uri))
//! }
//!
//! let app = Router::new()
//!     .route("/foo", get(|| async { /* ... */ }))
//!     .fallback(fallback.into_service());
//! # async {
//! # hyper::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! See [`Router::fallback`] for more details.
//!
//! ## Routing to any [`Service`]
//!
//! axum also supports routing to general [`Service`]s:
//!
//! ```rust,no_run
//! use axum::{
//!     Router,
//!     body::Body,
//!     routing::service_method_router as service,
//!     error_handling::HandleErrorExt,
//!     http::{Request, StatusCode},
//! };
//! use tower_http::services::ServeFile;
//! use http::Response;
//! use std::{convert::Infallible, io};
//! use tower::service_fn;
//!
//! let app = Router::new()
//!     .route(
//!         // Any request to `/` goes to a service
//!         "/",
//!         // Services who's response body is not `axum::body::BoxBody`
//!         // can be wrapped in `axum::service::any` (or one of the other routing filters)
//!         // to have the response body mapped
//!         service::any(service_fn(|_: Request<Body>| async {
//!             let res = Response::new(Body::from("Hi from `GET /`"));
//!             Ok::<_, Infallible>(res)
//!         }))
//!     )
//!     .route(
//!         "/foo",
//!         // This service's response body is `axum::body::BoxBody` so
//!         // it can be routed to directly.
//!         service_fn(|req: Request<Body>| async move {
//!             let body = Body::from(format!("Hi from `{} /foo`", req.method()));
//!             let body = axum::body::box_body(body);
//!             let res = Response::new(body);
//!             Ok::<_, Infallible>(res)
//!         })
//!     )
//!     .route(
//!         // GET `/static/Cargo.toml` goes to a service from tower-http
//!         "/static/Cargo.toml",
//!         service::get(ServeFile::new("Cargo.toml"))
//!             // though we must handle any potential errors
//!             .handle_error(|error: io::Error| {
//!                 (
//!                     StatusCode::INTERNAL_SERVER_ERROR,
//!                     format!("Unhandled internal error: {}", error),
//!                 )
//!             })
//!     );
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Routing to arbitrary services in this way has complications for backpressure
//! ([`Service::poll_ready`]). See the [`service`] module for more details.
//!
//! ### Routing to fallible services
//!
//! Note that routing to general services has a small gotcha when it comes to
//! errors. axum currently does not support mixing routes to fallible services
//! with infallible handlers. For example this does _not_ compile:
//!
//! ```compile_fail
//! use axum::{
//!     Router,
//!     routing::{get, service_method_router as service},
//!     http::{Request, Response},
//!     body::Body,
//! };
//! use std::io;
//! use tower::service_fn;
//!
//! let app = Router::new()
//!     // this route cannot fail
//!     .route("/foo", get(|| async {}))
//!     // this route can fail with io::Error
//!     .route(
//!         "/",
//!         service::get(service_fn(|_req: Request<Body>| async {
//!             let contents = tokio::fs::read_to_string("some_file").await?;
//!             Ok::<_, io::Error>(Response::new(Body::from(contents)))
//!         })),
//!     );
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! The solution is to use [`handle_error`] and handle the error from the
//! service:
//!
//! ```
//! use axum::{
//!     Router,
//!     body::Body,
//!     routing::{get, service_method_router as service},
//!     response::IntoResponse,
//!     http::{Request, Response},
//!     error_handling::HandleErrorExt,
//! };
//! use std::{io, convert::Infallible};
//! use tower::service_fn;
//!
//! let app = Router::new()
//!     // this route cannot fail
//!     .route("/foo", get(|| async {}))
//!     // this route can fail with io::Error
//!     .route(
//!         "/",
//!         service::get(service_fn(|_req: Request<Body>| async {
//!             let contents = tokio::fs::read_to_string("some_file").await?;
//!             Ok::<_, io::Error>(Response::new(Body::from(contents)))
//!         }))
//!         .handle_error(handle_io_error),
//!     );
//!
//! fn handle_io_error(error: io::Error) -> impl IntoResponse {
//!     // ...
//! }
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! In this particular case you can also handle the error directly in
//! `service_fn` but that is not possible, if you're routing to a service which
//! you don't control.
//!
//! See ["Error handling"](#error-handling) for more details on [`handle_error`]
//! and error handling in general.
//!
//! # Extractors
//!
//! An extractor is a type that implements [`FromRequest`]. Extractors is how
//! you pick apart the incoming request to get the parts your handler needs.
//!
//! For example, [`extract::Json`] is an extractor that consumes the request
//! body and deserializes it as JSON into some target type:
//!
//! ```rust,no_run
//! use axum::{
//!     extract::Json,
//!     routing::post,
//!     Router,
//! };
//! use serde::Deserialize;
//!
//! let app = Router::new().route("/users", post(create_user));
//!
//! #[derive(Deserialize)]
//! struct CreateUser {
//!     email: String,
//!     password: String,
//! }
//!
//! async fn create_user(Json(payload): Json<CreateUser>) {
//!     // ...
//! }
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! See the [`extract`] module for everything that can be used as an extractor.
//!
//! ## Common extractors
//!
//! Some commonly used extractors are:
//!
//! ```rust,no_run
//! use axum::{
//!     extract::{Json, TypedHeader, Path, Extension, Query},
//!     routing::post,
//!     http::{Request, header::HeaderMap},
//!     body::{Bytes, Body},
//!     Router,
//! };
//! use serde_json::Value;
//! use headers::UserAgent;
//! use std::collections::HashMap;
//!
//! // `Path` gives you the path parameters and deserializes them. See its docs for
//! // more details
//! async fn path(Path(user_id): Path<u32>) {}
//!
//! // `Query` gives you the query parameters and deserializes them.
//! async fn query(Query(params): Query<HashMap<String, String>>) {}
//!
//! // `HeaderMap` gives you all the headers
//! async fn headers(headers: HeaderMap) {}
//!
//! // `TypedHeader` can be used to extract a single header
//! // note this requires you've enabled axum's `headers`
//! async fn user_agent(TypedHeader(user_agent): TypedHeader<UserAgent>) {}
//!
//! // `String` consumes the request body and ensures it is valid utf-8
//! async fn string(body: String) {}
//!
//! // `Bytes` gives you the raw request body
//! async fn bytes(body: Bytes) {}
//!
//! // We've already seen `Json` for parsing the request body as json
//! async fn json(Json(payload): Json<Value>) {}
//!
//! // `Request` gives you the whole request for maximum control
//! async fn request(request: Request<Body>) {}
//!
//! // `Extension` extracts data from "request extensions"
//! // See the "Sharing state with handlers" section for more details
//! async fn extension(Extension(state): Extension<State>) {}
//!
//! #[derive(Clone)]
//! struct State { /* ... */ }
//!
//! let app = Router::new()
//!     .route("/path", post(path))
//!     .route("/query", post(query))
//!     .route("/user_agent", post(user_agent))
//!     .route("/headers", post(headers))
//!     .route("/string", post(string))
//!     .route("/bytes", post(bytes))
//!     .route("/json", post(json))
//!     .route("/request", post(request))
//!     .route("/extension", post(extension));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! ## Applying multiple extractors
//!
//! You can also apply multiple extractors:
//!
//! ```rust,no_run
//! use axum::{
//!     extract,
//!     routing::get,
//!     Router,
//! };
//! use uuid::Uuid;
//! use serde::Deserialize;
//!
//! let app = Router::new().route("/users/:id/things", get(get_user_things));
//!
//! #[derive(Deserialize)]
//! struct Pagination {
//!     page: usize,
//!     per_page: usize,
//! }
//!
//! impl Default for Pagination {
//!     fn default() -> Self {
//!         Self { page: 1, per_page: 30 }
//!     }
//! }
//!
//! async fn get_user_things(
//!     extract::Path(user_id): extract::Path<Uuid>,
//!     pagination: Option<extract::Query<Pagination>>,
//! ) {
//!     let pagination: Pagination = pagination.unwrap_or_default().0;
//!
//!     // ...
//! }
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Take care of the order in which you apply extractors as some extractors
//! mutate the request.
//!
//! For example using [`HeaderMap`] as an extractor will make the headers
//! inaccessible for other extractors on the handler. If you need to extract
//! individual headers _and_ a [`HeaderMap`] make sure to apply the extractor of
//! individual headers first:
//!
//! ```rust,no_run
//! use axum::{
//!     extract::TypedHeader,
//!     routing::get,
//!     http::header::HeaderMap,
//!     Router,
//! };
//! use headers::UserAgent;
//!
//! async fn handler(
//!     TypedHeader(user_agent): TypedHeader<UserAgent>,
//!     all_headers: HeaderMap,
//! ) {
//!     // ...
//! }
//!
//! let app = Router::new().route("/", get(handler));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Extractors that consume the request body can also only be applied once as
//! well as [`Request`], which consumes the entire request:
//!
//! ```rust,no_run
//! use axum::{
//!     routing::get,
//!     http::Request,
//!     body::Body,
//!     Router,
//! };
//!
//! async fn handler(request: Request<Body>) {
//!     // ...
//! }
//!
//! let app = Router::new().route("/", get(handler));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Extractors always run in the order of the function parameters that is from
//! left to right.
//!
//! ## Optional extractors
//!
//! All extractors defined in axum will reject the request if it doesn't match.
//! If you wish to make an extractor optional you can wrap it in `Option`:
//!
//! ```rust,no_run
//! use axum::{
//!     extract::Json,
//!     routing::post,
//!     Router,
//! };
//! use serde_json::Value;
//!
//! async fn create_user(payload: Option<Json<Value>>) {
//!     if let Some(payload) = payload {
//!         // We got a valid JSON payload
//!     } else {
//!         // Payload wasn't valid JSON
//!     }
//! }
//!
//! let app = Router::new().route("/users", post(create_user));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Wrapping extractors in `Result` makes them optional and gives you the reason
//! the extraction failed:
//!
//! ```rust,no_run
//! use axum::{
//!     extract::{Json, rejection::JsonRejection},
//!     routing::post,
//!     Router,
//! };
//! use serde_json::Value;
//!
//! async fn create_user(payload: Result<Json<Value>, JsonRejection>) {
//!     match payload {
//!         Ok(payload) => {
//!             // We got a valid JSON payload
//!         }
//!         Err(JsonRejection::MissingJsonContentType(_)) => {
//!             // Request didn't have `Content-Type: application/json`
//!             // header
//!         }
//!         Err(JsonRejection::InvalidJsonBody(_)) => {
//!             // Couldn't deserialize the body into the target type
//!         }
//!         Err(JsonRejection::BodyAlreadyExtracted(_)) => {
//!             // Another extractor had already consumed the body
//!         }
//!         Err(_) => {
//!             // `JsonRejection` is marked `#[non_exhaustive]` so match must
//!             // include a catch-all case.
//!         }
//!     }
//! }
//!
//! let app = Router::new().route("/users", post(create_user));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! ## Customizing extractor responses
//!
//! If an extractor fails it will return a response with the error and your
//! handler will not be called. To customize the error response you have a two
//! options:
//!
//! 1. Use `Result<T, T::Rejection>` as your extractor like shown in ["Optional
//!    extractors"](#optional-extractors). This works well if you're only using
//!    the extractor in a single handler.
//! 2. Create your own extractor that in its [`FromRequest`] implemention calls
//!    one of axum's built in extractors but returns a different response for
//!    rejections. See the [customize-extractor-error] example for more details.
//!
//! # Building responses
//!
//! Anything that implements [`IntoResponse`](response::IntoResponse) can be
//! returned from a handler:
//!
//! ```rust,no_run
//! use axum::{
//!     body::Body,
//!     routing::get,
//!     handler::Handler,
//!     http::{Request, header::{HeaderMap, HeaderName, HeaderValue}},
//!     response::{IntoResponse, Html, Json, Headers},
//!     Router,
//! };
//! use http::{StatusCode, Response, Uri};
//! use serde_json::{Value, json};
//!
//! // We've already seen returning &'static str
//! async fn plain_text() -> &'static str {
//!     "foo"
//! }
//!
//! // String works too and will get a `text/plain` content-type
//! async fn plain_text_string(uri: Uri) -> String {
//!     format!("Hi from {}", uri.path())
//! }
//!
//! // Bytes will get a `application/octet-stream` content-type
//! async fn bytes() -> Vec<u8> {
//!     vec![1, 2, 3, 4]
//! }
//!
//! // `()` gives an empty response
//! async fn empty() {}
//!
//! // `StatusCode` gives an empty response with that status code
//! async fn empty_with_status() -> StatusCode {
//!     StatusCode::NOT_FOUND
//! }
//!
//! // A tuple of `StatusCode` and something that implements `IntoResponse` can
//! // be used to override the status code
//! async fn with_status() -> (StatusCode, &'static str) {
//!     (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong")
//! }
//!
//! // A tuple of `HeaderMap` and something that implements `IntoResponse` can
//! // be used to override the headers
//! async fn with_headers() -> (HeaderMap, &'static str) {
//!     let mut headers = HeaderMap::new();
//!     headers.insert(
//!         HeaderName::from_static("x-foo"),
//!         HeaderValue::from_static("foo"),
//!     );
//!     (headers, "foo")
//! }
//!
//! // You can also override both status and headers at the same time
//! async fn with_headers_and_status() -> (StatusCode, HeaderMap, &'static str) {
//!     let mut headers = HeaderMap::new();
//!     headers.insert(
//!         HeaderName::from_static("x-foo"),
//!         HeaderValue::from_static("foo"),
//!     );
//!     (StatusCode::INTERNAL_SERVER_ERROR, headers, "foo")
//! }
//!
//! // `Headers` makes building the header map easier and `impl Trait` is easier
//! // so you don't have to write the whole type
//! async fn with_easy_headers() -> impl IntoResponse {
//!     Headers(vec![("x-foo", "foo")])
//! }
//!
//! // `Html` gives a content-type of `text/html`
//! async fn html() -> Html<&'static str> {
//!     Html("<h1>Hello, World!</h1>")
//! }
//!
//! // `Json` gives a content-type of `application/json` and works with any type
//! // that implements `serde::Serialize`
//! async fn json() -> Json<Value> {
//!     Json(json!({ "data": 42 }))
//! }
//!
//! // `Result<T, E>` where `T` and `E` implement `IntoResponse` is useful for
//! // returning errors
//! async fn result() -> Result<&'static str, StatusCode> {
//!     Ok("all good")
//! }
//!
//! // `Response` gives full control
//! async fn response() -> Response<Body> {
//!     Response::builder().body(Body::empty()).unwrap()
//! }
//!
//! let app = Router::new()
//!     .route("/plain_text", get(plain_text))
//!     .route("/plain_text_string", get(plain_text_string))
//!     .route("/bytes", get(bytes))
//!     .route("/empty", get(empty))
//!     .route("/empty_with_status", get(empty_with_status))
//!     .route("/with_status", get(with_status))
//!     .route("/with_headers", get(with_headers))
//!     .route("/with_headers_and_status", get(with_headers_and_status))
//!     .route("/with_easy_headers", get(with_easy_headers))
//!     .route("/html", get(html))
//!     .route("/json", get(json))
//!     .route("/result", get(result))
//!     .route("/response", get(response));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! # Error handling
//!
//! In the context of axum an "error" specifically means if a [`Service`]'s
//! response future resolves to `Err(Service::Error)`. That means async handler
//! functions can _never_ fail since they always produce a response and their
//! `Service::Error` type is [`Infallible`]. Returning statuses like 404 or 500
//! are _not_ errors.
//!
//! axum works this way because hyper will close the connection, without sending
//! a response, if an error is encountered. This is not desireable so axum makes
//! it impossible to forget to handle errors.
//!
//! Sometimes you need to route to fallible services or apply fallible
//! middleware in which case you need to handle the errors. That can be done
//! using things from [`error_handling`].
//!
//! You can find examples here:
//! - [Routing to fallible services](#routing-to-fallible-services)
//! - [Applying fallible middleware](#applying-multiple-middleware)
//!
//! # Applying middleware
//!
//! axum is designed to take full advantage of the tower and tower-http
//! ecosystem of middleware.
//!
//! If you're new to tower we recommend you read its [guides][tower-guides] for
//! a general introduction to tower and its concepts.
//!
//! ## To individual handlers
//!
//! A middleware can be applied to a single handler like so:
//!
//! ```rust,no_run
//! use axum::{
//!     handler::Handler,
//!     routing::get,
//!     Router,
//! };
//! use tower::limit::ConcurrencyLimitLayer;
//!
//! let app = Router::new()
//!     .route(
//!         "/",
//!         get(handler.layer(ConcurrencyLimitLayer::new(100))),
//!     );
//!
//! async fn handler() {}
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! ## To groups of routes
//!
//! Middleware can also be applied to a group of routes like so:
//!
//! ```rust,no_run
//! use axum::{
//!     routing::{get, post},
//!     Router,
//! };
//! use tower::limit::ConcurrencyLimitLayer;
//!
//! async fn handler() {}
//!
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .route("/foo", post(handler))
//!     .layer(ConcurrencyLimitLayer::new(100));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Note that [`Router::layer`] applies the middleware to all previously added
//! routes, of that particular `Router`. If you need multiple groups of routes
//! with different middleware build them separately and combine them with
//! [`Router::merge`]:
//!
//! ```rust,no_run
//! use axum::{
//!     routing::{get, post},
//!     Router,
//! };
//! use tower::limit::ConcurrencyLimitLayer;
//! # type MyAuthLayer = tower::layer::util::Identity;
//!
//! async fn handler() {}
//!
//! let foo = Router::new()
//!     .route("/", get(handler))
//!     .route("/foo", post(handler))
//!     .layer(ConcurrencyLimitLayer::new(100));
//!
//! let bar = Router::new()
//!     .route("/requires-auth", get(handler))
//!     .layer(MyAuthLayer::new());
//!
//! let app = foo.merge(bar);
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! ## Applying multiple middleware
//!
//! [`tower::ServiceBuilder`] can be used to combine multiple middleware:
//!
//! ```rust,no_run
//! use axum::{
//!     body::Body,
//!     routing::get,
//!     http::{Request, StatusCode},
//!     error_handling::HandleErrorLayer,
//!     response::IntoResponse,
//!     Router, BoxError,
//! };
//! use tower::ServiceBuilder;
//! use tower_http::compression::CompressionLayer;
//! use std::{borrow::Cow, time::Duration};
//!
//! let middleware_stack = ServiceBuilder::new()
//!     // Handle errors from middleware
//!     //
//!     // This middleware most be added above any fallible
//!     // ones if you're using `ServiceBuilder`, due to how ordering works
//!     .layer(HandleErrorLayer::new(handle_error))
//!     // Return an error after 30 seconds
//!     .timeout(Duration::from_secs(30))
//!     // Shed load if we're receiving too many requests
//!     .load_shed()
//!     // Process at most 100 requests concurrently
//!     .concurrency_limit(100)
//!     // Compress response bodies
//!     .layer(CompressionLayer::new());
//!
//! let app = Router::new()
//!     .route("/", get(|_: Request<Body>| async { /* ... */ }))
//!     .layer(middleware_stack);
//!
//! fn handle_error(error: BoxError) -> impl IntoResponse {
//!     (
//!         StatusCode::INTERNAL_SERVER_ERROR,
//!         format!("Something went wrong: {}", error),
//!     )
//! }
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! See [Error handling](#error-handling) for more details on general error handling in axum.
//!
//! ## Commonly used middleware
//!
//! [`tower::util`] and [`tower_http`] have a large collection of middleware that are compatible
//! with axum. Some commonly used are:
//!
//! ```rust,no_run
//! use axum::{
//!     body::{Body, BoxBody},
//!     routing::get,
//!     http::{Request, Response},
//!     error_handling::HandleErrorLayer,
//!     Router,
//! };
//! use tower::{
//!     filter::AsyncFilterLayer,
//!     util::AndThenLayer,
//!     ServiceBuilder,
//! };
//! use std::convert::Infallible;
//! use tower_http::trace::TraceLayer;
//! #
//! # fn handle_error<T>(error: T) -> axum::http::StatusCode {
//! #     axum::http::StatusCode::INTERNAL_SERVER_ERROR
//! # }
//!
//! let middleware_stack = ServiceBuilder::new()
//!     // Handle errors from middleware
//!     //
//!     // This middleware most be added above any fallible
//!     // ones if you're using `ServiceBuilder`, due to how ordering works
//!     .layer(HandleErrorLayer::new(handle_error))
//!     // `TraceLayer` adds high level tracing and logging
//!     .layer(TraceLayer::new_for_http())
//!     // `AsyncFilterLayer` lets you asynchronously transform the request
//!     .layer(AsyncFilterLayer::new(map_request))
//!     // `AndThenLayer` lets you asynchronously transform the response
//!     .layer(AndThenLayer::new(map_response));
//!
//! async fn map_request(req: Request<Body>) -> Result<Request<Body>, Infallible> {
//!     Ok(req)
//! }
//!
//! async fn map_response(res: Response<BoxBody>) -> Result<Response<BoxBody>, Infallible> {
//!     Ok(res)
//! }
//!
//! let app = Router::new()
//!     .route("/", get(|| async { /* ... */ }))
//!     .layer(middleware_stack);
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! Additionally axum provides [`extract::extractor_middleware()`] for converting any extractor into
//! a middleware. Among other things, this can be useful for doing authorization. See
//! [`extract::extractor_middleware()`] for more details.
//!
//! See [Error handling](#error-handling) for more details on general error handling in axum.
//!
//! ## Writing your own middleware
//!
//! You can also write you own middleware by implementing [`tower::Service`]:
//!
//! ```
//! use axum::{
//!     body::{Body, BoxBody},
//!     routing::get,
//!     http::{Request, Response},
//!     Router,
//! };
//! use futures::future::BoxFuture;
//! use tower::{Service, layer::layer_fn};
//! use std::task::{Context, Poll};
//!
//! #[derive(Clone)]
//! struct MyMiddleware<S> {
//!     inner: S,
//! }
//!
//! impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for MyMiddleware<S>
//! where
//!     S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
//!     S::Future: Send + 'static,
//!     ReqBody: Send + 'static,
//!     ResBody: Send + 'static,
//! {
//!     type Response = S::Response;
//!     type Error = S::Error;
//!     type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
//!
//!     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//!         self.inner.poll_ready(cx)
//!     }
//!
//!     fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
//!         println!("`MyMiddleware` called!");
//!
//!         // best practice is to clone the inner service like this
//!         // see https://github.com/tower-rs/tower/issues/547 for details
//!         let clone = self.inner.clone();
//!         let mut inner = std::mem::replace(&mut self.inner, clone);
//!
//!         Box::pin(async move {
//!             let res: Response<ResBody> = inner.call(req).await?;
//!
//!             println!("`MyMiddleware` received the response");
//!
//!             Ok(res)
//!         })
//!     }
//! }
//!
//! let app = Router::new()
//!     .route("/", get(|| async { /* ... */ }))
//!     .layer(layer_fn(|inner| MyMiddleware { inner }));
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! # Sharing state with handlers
//!
//! It is common to share some state between handlers for example to share a
//! pool of database connections or clients to other services. That can be done
//! using the [`AddExtension`] middleware (applied with [`AddExtensionLayer`])
//! and the [`extract::Extension`] extractor:
//!
//! ```rust,no_run
//! use axum::{
//!     AddExtensionLayer,
//!     extract,
//!     routing::get,
//!     Router,
//! };
//! use std::sync::Arc;
//!
//! struct State {
//!     // ...
//! }
//!
//! let shared_state = Arc::new(State { /* ... */ });
//!
//! let app = Router::new()
//!     .route("/", get(handler))
//!     .layer(AddExtensionLayer::new(shared_state));
//!
//! async fn handler(
//!     state: extract::Extension<Arc<State>>,
//! ) {
//!     let state: Arc<State> = state.0;
//!
//!     // ...
//! }
//! # async {
//! # axum::Server::bind(&"".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
//! # };
//! ```
//!
//! # Required dependencies
//!
//! To use axum there are a few dependencies you have pull in as well:
//!
//! ```toml
//! [dependencies]
//! axum = "<latest-version>"
//! hyper = { version = "<latest-version>", features = ["full"] }
//! tokio = { version = "<latest-version>", features = ["full"] }
//! tower = "<latest-version>"
//! ```
//!
//! The `"full"` feature for hyper and tokio isn't strictly necessary but its
//! the easiest way to get started.
//!
//! Note that [`hyper::Server`] is re-exported by axum so if thats all you need
//! then you don't have to explicitly depend on hyper.
//!
//! Tower isn't strictly necessary either but helpful for testing. See the
//! testing example in the repo to learn more about testing axum apps.
//!
//! # Examples
//!
//! The axum repo contains [a number of examples][examples] that show how to put all the
//! pieces together.
//!
//! # Feature flags
//!
//! axum uses a set of [feature flags] to reduce the amount of compiled and
//! optional dependencies.
//!
//! The following optional features are available:
//!
//! - `headers`: Enables extracting typed headers via [`extract::TypedHeader`].
//! - `http1`: Enables hyper's `http1` feature. Enabled by default.
//! - `http2`: Enables hyper's `http2` feature.
//! - `json`: Enables the [`Json`] type and some similar convenience functionality.
//!   Enabled by default.
//! - `multipart`: Enables parsing `multipart/form-data` requests with [`extract::Multipart`].
//! - `tower-log`: Enables `tower`'s `log` feature. Enabled by default.
//! - `ws`: Enables WebSockets support via [`extract::ws`].
//!
//! [`tower`]: https://crates.io/crates/tower
//! [`tower-http`]: https://crates.io/crates/tower-http
//! [`tokio`]: http://crates.io/crates/tokio
//! [`hyper`]: http://crates.io/crates/hyper
//! [`tonic`]: http://crates.io/crates/tonic
//! [feature flags]: https://doc.rust-lang.org/cargo/reference/features.html#the-features-section
//! [`IntoResponse`]: crate::response::IntoResponse
//! [`Timeout`]: tower::timeout::Timeout
//! [examples]: https://github.com/tokio-rs/axum/tree/main/examples
//! [`Router::merge`]: crate::routing::Router::merge
//! [`axum::Server`]: hyper::server::Server
//! [`OriginalUri`]: crate::extract::OriginalUri
//! [`Service`]: tower::Service
//! [`Service::poll_ready`]: tower::Service::poll_ready
//! [`tower::Service`]: tower::Service
//! [`handle_error`]: error_handling::HandleErrorExt::handle_error
//! [tower-guides]: https://github.com/tower-rs/tower/tree/master/guides
//! [`Uuid`]: https://docs.rs/uuid/latest/uuid/
//! [`FromRequest`]: crate::extract::FromRequest
//! [`HeaderMap`]: http::header::HeaderMap
//! [`Request`]: http::Request
//! [customize-extractor-error]: https://github.com/tokio-rs/axum/blob/main/examples/customize-extractor-error/src/main.rs
//! [axum-debug]: https://docs.rs/axum-debug
//! [`debug_handler`]: https://docs.rs/axum-debug/latest/axum_debug/attr.debug_handler.html
//! [`Handler`]: crate::handler::Handler
//! [`Infallible`]: std::convert::Infallible

#![warn(
    clippy::all,
    clippy::dbg_macro,
    clippy::todo,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::mem_forget,
    clippy::unused_self,
    clippy::filter_map_next,
    clippy::needless_continue,
    clippy::needless_borrow,
    clippy::match_wildcard_for_single_variants,
    clippy::if_let_mutex,
    clippy::mismatched_target_os,
    clippy::await_holding_lock,
    clippy::match_on_vec_items,
    clippy::imprecise_flops,
    clippy::suboptimal_flops,
    clippy::lossy_float_literal,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::fn_params_excessive_bools,
    clippy::exit,
    clippy::inefficient_to_string,
    clippy::linkedlist,
    clippy::macro_use_imports,
    clippy::option_option,
    clippy::verbose_file_reads,
    clippy::unnested_or_patterns,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    missing_debug_implementations,
    missing_docs
)]
#![deny(unreachable_pub, private_in_public)]
#![allow(elided_lifetimes_in_paths, clippy::type_complexity)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]

#[macro_use]
pub(crate) mod macros;

mod add_extension;
mod clone_box_service;
mod error;
#[cfg(feature = "json")]
mod json;
mod util;

pub mod body;
pub mod error_handling;
pub mod extract;
pub mod handler;
pub mod response;
pub mod routing;

#[cfg(test)]
mod tests;

pub use add_extension::{AddExtension, AddExtensionLayer};
#[doc(no_inline)]
pub use async_trait::async_trait;
#[doc(no_inline)]
pub use http;
#[doc(no_inline)]
pub use hyper::Server;

#[doc(inline)]
#[cfg(feature = "json")]
pub use self::json::Json;
#[doc(inline)]
pub use self::{error::Error, routing::Router};

/// Alias for a type-erased error type.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
