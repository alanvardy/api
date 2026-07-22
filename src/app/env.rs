//! Stores all the environment variables and verifies that they are available at startup
//! Set them for production with `fly secrets set KEY=VALUE`
//! Set them locally in `.env`

use aws_config::SdkConfig;

#[derive(Clone)]
pub struct Env {
    pub feature_flags_web_password: String,
    pub aws_config: SdkConfig,
    pub s3_bucket: String,
    pub http_port: u16,
    pub sentry_dsn: String,
    pub bearer_token: String,
}

impl Env {
    pub async fn init() -> Env {
        let aws_config = aws_config::load_from_env().await;
        let s3_bucket = get_env("BUCKET_NAME");
        let sentry_dsn = get_env("SENTRY_DSN");
        let http_port = get_env("PORT").parse().unwrap();
        let feature_flags_web_password = get_env("FEATURE_FLAGS_WEB_PASSWORD").parse().unwrap();
        let bearer_token = get_env("BEARER_TOKEN");

        Env {
            aws_config,
            sentry_dsn,
            feature_flags_web_password,
            s3_bucket,
            http_port,
            bearer_token,
        }
    }
}

fn get_env(key: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|var| !var.is_empty())
        .unwrap_or_else(|| panic!("{key} must be set and non-empty"))
}
