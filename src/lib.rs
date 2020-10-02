//! structured request logging middleware
//!
//! ```no_run
//! use actix_web::{HttpServer, App};
//! use actix_slog::StructuredLogger;
//! use slog::o;
//!
//! let logger: slog::Logger = unimplemented!();
//! let server = HttpServer::new(move || {
//!   App::new()
//!     .wrap(
//!       StructuredLogger::new(logger.new(o!("log_type" => "access"))),
//!     )
//!   })
//!   .bind("[::1]:8080");
//! ```
use actix_web::dev::{
    BodySize, MessageBody, ResponseBody, Service, ServiceRequest, ServiceResponse, Transform,
};
use actix_web::error::{Error, Result};
use actix_web::http::header::{HOST, REFERER, USER_AGENT};
use actix_web::http::StatusCode;
use actix_web::web::Bytes;
use chrono::prelude::*;
use futures::future::{ok, Ready};
use pin_project::{pin_project, pinned_drop};
use slog::{debug, info, o, Logger};
use std::borrow::ToOwned;
use std::collections::HashSet;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// global configuration/builder for the log middleware
pub struct StructuredLogger(Rc<Inner>);

struct Inner {
    logger: Logger,
    exclude: HashSet<String>,
}

impl StructuredLogger {
    /// Create `Logger` middleware with the specified `format`.
    #[must_use]
    pub fn new(logger: Logger) -> StructuredLogger {
        StructuredLogger(Rc::new(Inner {
            logger,
            exclude: HashSet::new(),
        }))
    }

    /// Ignore and do not log access for specified path.
    pub fn exclude<T: Into<String>>(mut self, path: T) -> Self {
        Rc::get_mut(&mut self.0)
            .unwrap()
            .exclude
            .insert(path.into());
        self
    }
}

/// "initializer" for the service/the actual middleware (called once per worker)
impl<S, B> Transform<S> for StructuredLogger
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<StreamLog<B>>;
    type Error = Error;
    type Transform = StructuredLoggerMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(StructuredLoggerMiddleware {
            service,
            inner: self.0.clone(),
        })
    }
}

/// Logger middleware
pub struct StructuredLoggerMiddleware<S> {
    inner: Rc<Inner>,

    /// the next service in the chain, kind of like express' next()
    service: S,
}

impl<S, B> Service for StructuredLoggerMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<StreamLog<B>>;
    type Error = Error;
    type Future = LoggerResponse<S, B>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        // check the exclude-list if to skip this path…
        let is_exclude = self.inner.exclude.contains(req.path());

        // …but collect other fields nevertheless, to log errors etc.
        let timestamp = Utc::now();

        let user_agent = req
            .headers()
            .get(USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");

        let referer = req
            .headers()
            .get(REFERER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");

        let remote_addr = req
            .connection_info()
            .remote_addr()
            .map_or(String::from("-"), ToOwned::to_owned);

        let host = req
            .headers()
            .get(HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");

        let correlation_id = req
            .headers()
            .get("correlation-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");

        let logger = self.inner.logger.new(o!(
            "http_version" => format!("{:?}", req.version()),
            "http_host" => host.to_owned(),
            "referer" => referer.to_owned(),
            "remote_address" => remote_addr,
            "user-agent" => user_agent.to_owned(),
            "request_method" => req.method().to_string(),
            "correlation_id" => correlation_id.to_owned(),
            "uri" => req.path().to_owned(),
            "query" => format!("?{}", req.query_string()),
        ));

        LoggerResponse {
            logger,
            fut: self.service.call(req),
            timestamp,
            _t: PhantomData,
            is_exclude,
        }
    }
}

#[doc(hidden)]
#[pin_project::pin_project]
pub struct LoggerResponse<S, B>
where
    B: MessageBody,
    S: Service,
{
    #[pin]
    fut: S::Future,
    // timestamp at which the request hit the service (in contrast to when the log is written, i.e. the request is done)
    timestamp: DateTime<Utc>,
    logger: Logger,
    // if to exclude this request
    is_exclude: bool,
    _t: PhantomData<(B,)>,
}

/// "handler" for the response, i.e. "action" to call once the other services are done, and the
/// response is ready
impl<S, B> Future for LoggerResponse<S, B>
where
    B: MessageBody,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Output = Result<ServiceResponse<StreamLog<B>>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();

        let res = match futures::ready!(this.fut.poll(cx)) {
            Ok(res) => res,
            Err(e) => return Poll::Ready(Err(e)),
        };

        if let Some(error) = res.response().error() {
            if res.response().head().status != StatusCode::INTERNAL_SERVER_ERROR {
                debug!(this.logger, "Error in response: {:?}", error);
            }
        }

        let timestamp = *this.timestamp;
        let logger = this.logger.new(o!("status" => res.status().as_u16()));
        let is_exclude: bool = *this.is_exclude;

        Poll::Ready(Ok(res.map_body(move |_, body| {
            ResponseBody::Body(StreamLog {
                logger,
                is_exclude,
                body,
                timestamp,
                size: 0,
            })
        })))
    }
}

#[pin_project(PinnedDrop)]
pub struct StreamLog<B> {
    logger: Logger,
    is_exclude: bool,
    #[pin]
    body: ResponseBody<B>,
    size: usize,
    timestamp: DateTime<Utc>,
}

#[pinned_drop]
impl<B> PinnedDrop for StreamLog<B> {
    fn drop(self: Pin<&mut Self>) {
        if !self.is_exclude {
            let response_time = Utc::now() - self.timestamp;
            let response_time = response_time.num_milliseconds();
            info!(self.logger, "-"; o!("bytes_sent" => self.size), "response_time" => response_time);
        }
    }
}

impl<B: MessageBody> MessageBody for StreamLog<B> {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Bytes, Error>>> {
        let this = self.project();
        match this.body.poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                *this.size += chunk.len();
                Poll::Ready(Some(Ok(chunk)))
            }
            val => val,
        }
    }
}
