use axum::{Router, routing::get};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use std::panic::{PanicHookInfo, set_hook};
use tracing::{error, error_span};

fn traced_panic_hook(info: &PanicHookInfo) {
    metrics::counter!("service_panics").increment(1);
    let location = info.location();

    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<dyn Any>",
        },
    };

    let thread = std::thread::current();
    let thread = thread.name().unwrap_or("<unnamed>");
    let backtrace = std::backtrace::Backtrace::capture();

    let _entered = if let Some(location) = location {
        error_span!("panic", %thread, location = %format!("{}:{}:{}", location.file(), location.line(), location.column()))
    } else {
        // very unlikely to hit here, but the guarantees of std could change
        error_span!("panic", %thread)
    }
    .entered();

    if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
        error!(backtrace=%backtrace, "{msg}");
    } else {
        error!("{msg}");
    }
}

#[tokio::main]
async fn main() {
    let _guard = init_tracing_opentelemetry::TracingConfig::default()
        .with_json_format()
        .with_stderr()
        .with_log_directives("debug")
        .init_subscriber()
        .expect("Failed to setup otel");

    set_hook(Box::new(traced_panic_hook));

    let app = Router::new()
        .route("/", get(handler))
        .layer(OtelInResponseLayer)
        .layer(OtelAxumLayer::default());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> String {
    unimplemented!("Oh no");
}
