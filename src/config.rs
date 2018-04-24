
use clap::App;
use std::env;

#[derive(Clone)]
pub struct QuiViveConfig {
    pub external_url: String,
    pub listener_url: String,
    pub redis_hostname: Option<String>,
    pub redis_password: Option<String>,
    pub cache_type: Option<String>,
}

impl QuiViveConfig {

    pub fn new() -> Self {
        QuiViveConfig {
            external_url: "".to_string(),
            listener_url: "".to_string(),
            redis_hostname: None,
            redis_password: None,
            cache_type: None,
        }
    }

    pub fn load_cli(&mut self) {
        let yaml = load_yaml!("cli.yml");
        let app = App::from_yaml(yaml);
        let matches = app.version(crate_version!()).get_matches();

        self.listener_url = matches.value_of("listener-url").unwrap_or("127.0.0.1:8080").to_string();
        self.external_url = matches.value_of("external-url").unwrap_or(self.listener_url.as_ref()).to_string();

        let redis_hostname = if let Some(value) = matches.value_of("redis-hostname") {
            Some(String::from(value))
        } else {
            None
        };

        let redis_password = if let Some(value) = matches.value_of("redis-password") {
            Some(String::from(value))
        } else {
            None
        };

        self.redis_hostname = redis_hostname;
        self.redis_password = redis_password;

        let cache_type = if let Some(value) = matches.value_of("cache-type") {
            Some(String::from(value))
        } else {
            None
        };

        self.cache_type = cache_type;
    }

    pub fn load_env(&mut self) {
        if let Ok(val) = env::var("EXTERNAL_URL") {
            self.external_url = Some(val).unwrap();
        }

        if let Ok(val) = env::var("LISTENER_URL") {
            self.listener_url = Some(val).unwrap();
        }

        if let Ok(val) = env::var("REDIS_HOSTNAME") {
            self.redis_hostname = Some(val);
        }

        if let Ok(val) = env::var("REDIS_PASSWORD") {
            self.redis_password = Some(val);
        }

        if let Ok(val) = env::var("CACHE_TYPE") {
            self.cache_type = Some(val);
        }
    }
}
