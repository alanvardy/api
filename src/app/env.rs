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
}

impl Env {
    pub async fn init() -> Env {
        let feature_flags_web_password = std::env::var("FEATURE_FLAGS_WEB_PASSWORD")
            .ok()
            .filter(|password| !password.is_empty())
            .expect("FEATURE_FLAGS_WEB_PASSWORD must be set and non-empty");
        let aws_config = aws_config::load_from_env().await;
        let s3_bucket = std::env::var("BUCKET_NAME")
            .ok()
            .filter(|bucket| !bucket.is_empty())
            .expect("BUCKET_NAME must be set and non-empty");

        let http_port = std::env::var("PORT")
            .ok()
            .filter(|var| !var.is_empty())
            .expect("PORT must be set and non-empty")
            .parse()
            .unwrap();

        Env {
            aws_config,
            feature_flags_web_password,
            s3_bucket,
            http_port,
        }
    }
}
