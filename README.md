[![Rust build](https://github.com/elmarx/actix-slog/workflows/Rust/badge.svg)](https://github.com/elmarx/actix-slog/actions?query=workflow%3ARust) [![crates.io badge](https://img.shields.io/crates/v/actix-slog.svg)](https://crates.io/crates/actix-slog) [![docs.rs badge](https://docs.rs/actix-slog/badge.svg)](https://docs.rs/actix-slog)

# Structured (access-) logging for actix-web

Provides a [middleware](https://docs.rs/actix-web/2.0.0/actix_web/struct.App.html#method.wrap) (`StructuredLogger`), 
similar to [actix_web::middleware:Logger](https://docs.rs/actix-web/2.0.0/actix_web/middleware/struct.Logger.html),
except that it uses [slog](https://crates.io/crates/slog) and thus enables JSON-formatted logging (via [slog-json](https://crates.io/crates/slog-json)).

Of course [slog's compact terminal output](https://github.com/slog-rs/slog#terminal-output-example) is a nice add-on, 
even if you're just out for JSON-logging.

## Usage

See [server_json](examples/server_compact.rs) and [server_compact](examples/server_compact.rs) for working examples.

```rust
  let logger: slog::Logger = unimplemented!();

  HttpServer::new(move || {
    App::new()
      .wrap(
        StructuredLogger::new(logger.new(o!("log_type" => "access"))),
      )
    })
    .bind("[::1]:8080")
```
