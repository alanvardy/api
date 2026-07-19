//! Stores all the environment variables and verifies that they are available at startup
//! Set them for production with `fly secrets set KEY=VALUE`
//! Set them locally in `.env`

pub struct Env {
    pub feature_flags_web_password: String,
}

impl Env {
    pub fn init() -> Env {
        let feature_flags_web_password = std::env::var("FEATURE_FLAGS_WEB_PASSWORD")
            .ok()
            .filter(|password| !password.is_empty())
            .expect("FEATURE_FLAGS_WEB_PASSWORD must be set and non-empty");

        Env {
            feature_flags_web_password,
        }
    }
}
