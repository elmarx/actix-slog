use actix_slog::StructuredLogger;
use actix_web::{get, App, HttpRequest, HttpResponse, HttpServer, Responder};
use slog::o;
use slog::Drain;
use slog::FnValue;
use slog_term::{CompactFormat, TermDecorator};

#[get("/")]
pub async fn index(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[get("/a")]
pub async fn a(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[get("/b")]
pub async fn b(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[get("/c")]
pub async fn c(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // TermDecorator with CompactFormat is probably nicer for (local) development
    let decorator = TermDecorator::new().build();
    let drain = CompactFormat::new(decorator).build().fuse();

    // although not strictly required, using slog_async is always a good idea to unblock the main-thread
    // otherwise it would block until logging has been completed
    let drain = slog_async::Async::new(drain).build().fuse();

    // generic root-logger you can use for the whole application
    let root_logger = slog::Logger::root(
        drain,
        o!("version" => env!("CARGO_PKG_VERSION"), "module" => FnValue(move |info| {
            info.module().to_string()
        })
        ),
    );

    HttpServer::new(move || {
        App::new()
            .wrap(
                // initialize the structured access-logger with a child-logger/scoped logger
                StructuredLogger::new(root_logger.new(o!("log_type" => "access"))),
            )
            .service(index)
            .service(a)
            .service(b)
            .service(c)
    })
    .bind("[::1]:8080")?
    .run()
    .await
}
