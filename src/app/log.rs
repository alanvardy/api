use axum::extract::{MatchedPath, Request};
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, Span};
use tracing_subscriber::{EnvFilter, fmt};

type MakeSpanFn = fn(&Request) -> Span;

// Emit one structured JSON log line per event so Fly.io's stdout capture can
// forward request logs to downstream aggregators such as Loki/Grafana.
pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));

    fmt()
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .with_env_filter(filter)
        .init();
}

// Log the matched route (e.g. `/users/{id}`) rather than the concrete path
// so per-request logs stay low cardinality and group cleanly in Grafana.
pub fn trace_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>, MakeSpanFn> {
    TraceLayer::new_for_http()
        .make_span_with(make_span as MakeSpanFn)
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
}

fn make_span(request: &Request) -> Span {
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path());

    tracing::info_span!(
        "http_request",
        method = %request.method(),
        path = %path,
    )
}
