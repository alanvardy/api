use crate::app::env::Env;

pub fn init(env: &Env) -> sentry::ClientInitGuard {
    sentry::init((
        env.sentry_dsn.clone(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            // Capture user IPs and potentially sensitive headers when using HTTP server integrations
            // see https://docs.sentry.io/platforms/rust/data-management/data-collected for more info
            send_default_pii: true,
            ..Default::default()
        },
    ))
}
