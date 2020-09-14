use actix_slog::StructuredLogger;
use actix_web::{get, App, HttpRequest, HttpResponse, HttpServer, Responder};
use chrono::{Local, SecondsFormat};
use slog::o;
use slog::Drain;
use slog::{FnValue, PushFnValue};
use slog_json::Json;

const SERVICE_NAME: &str = "sample_httpd";

#[get("/liveness")]
pub async fn liveness(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
}

#[get("/")]
pub async fn index(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let drain = Json::new(std::io::stdout())
        .add_key_value(o!(
        "@timestamp" => PushFnValue(move |_, ser| {
            ser.emit(Local::now().to_rfc3339_opts(SecondsFormat::Secs, true))
        }),
        "loglevel" => FnValue(move |rinfo| {
            rinfo.level().as_str()
        }),
        "msg" => PushFnValue(move |record, ser| {
            ser.emit(record.msg())
        }),
        ))
        .build()
        .fuse();

    // although not strictly required, using slog_async is always a good idea to unblock the main-thread
    // otherwise it would block until logging has been completed
    let drain = slog_async::Async::new(drain).build().fuse();

    // generic root-logger you can use for the whole application
    let root_logger = slog::Logger::root(
        drain,
        o!("version" => env!("CARGO_PKG_VERSION"), "service" => SERVICE_NAME, "log_type" => "application", "application_type" => "service", "module" => FnValue(move |info| {
            info.module().to_string()
        })
        ),
    );

    HttpServer::new(move || {
        App::new()
            .wrap(
                // initialize the structured access-logger with a child-logger/scoped logger
                StructuredLogger::new(root_logger.new(
                    o!("log_type" => "access", "protocol" => "http", "server_name" => SERVICE_NAME),
                ))
                // exclude access to some endpoints, e.g. liveness-probes (in kubernetes-context), or metrics
                // depends on your preference, maybe it also makes sense to NOT exclude those paths
                .exclude("/liveness"),
            )
            .service(liveness)
            .service(index)
    })
    .bind("[::1]:8080")?
    .run()
    .await
}
