
use clap::App;
use std::env;

#[derive(Clone,PartialEq)]
pub enum CustomIdFormat {
    None,
    Uuid,
    All,
}

#[derive(Clone)]
pub struct QuiViveConfig {
    pub external_url: String,
    pub listener_url: String,
    pub redis_hostname: Option<String>,
    pub redis_password: Option<String>,
    pub cache_type: Option<String>,
    pub id_length: u32,
    pub id_charset: String,
    pub custom_id_format: CustomIdFormat,
    pub default_expiration: Option<u32>,
    pub max_value_size: usize,
}

const ID_LENGTH: u32 = 12;

const ID_CHARSET: &str = "23456789\
            abcdefghjkimnpqrstuvwxyz\
            ABCDEFGHJKLMNPQRSTUVWXYZ";

impl QuiViveConfig {

    pub fn new() -> Self {
        QuiViveConfig {
            external_url: "".to_string(),
            listener_url: "".to_string(),
            redis_hostname: None,
            redis_password: None,
            cache_type: None,
            id_length: ID_LENGTH,
            id_charset: ID_CHARSET.to_string(),
            custom_id_format: CustomIdFormat::All,
            default_expiration: Some(86400), // 24 hours
            max_value_size: 1024*1024 // 1MB
        }
    }

    pub fn load_cli(&mut self) {
        let yaml = load_yaml!("cli.yml");
        let app = App::from_yaml(yaml);
        let matches = app.version(crate_version!()).get_matches();

        self.listener_url = matches.value_of("listener-url").unwrap_or("http://0.0.0.0:8080").to_string();
        self.external_url = matches.value_of("external-url").unwrap_or("http://127.0.0.1:8080").to_string();

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

        self.id_length = value_t!(matches, "id-length", u32).unwrap_or(9);

        if let Some(id_charset) = matches.value_of("id-charset") {
            self.id_charset = id_charset.to_string();
        }

        if let Some(default_expiration) = matches.value_of("default-expiration") {
            if let Ok(default_expiration) = default_expiration.parse::<u32>() {
                self.default_expiration = if default_expiration == 0 {
                    None
                } else {
                    Some(default_expiration)
                };
            }
        }

        if let Some(custom_id_format) = matches.value_of("custom-id-format") {
            self.custom_id_format = match custom_id_format.to_lowercase().as_str() {
                "none" => CustomIdFormat::None,
                "uuid" => CustomIdFormat::Uuid,
                "all" => CustomIdFormat::All,
                _ => CustomIdFormat::All,
            }
        }

        if let Some(max_value_size) = matches.value_of("max-value-size") {
            if let Ok(max_value_size) = max_value_size.parse::<usize>() {
                self.max_value_size = max_value_size;
            }
        }
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

        if let Ok(val) = env::var("ID_LENGTH") {
            if let Ok(id_length) = Some(val).unwrap().parse::<u32>() {
                self.id_length = id_length;
            }
        }

        if let Ok(val) = env::var("ID_CHARSET") {
            self.id_charset = Some(val).unwrap();
        }

        if let Ok(val) = env::var("DEFAULT_EXPIRATION") {
            if let Ok(default_expiration) = Some(val).unwrap().parse::<u32>() {
                self.default_expiration = if default_expiration == 0 {
                    None
                } else {
                    Some(default_expiration)
                };
            }
        }

        if let Ok(val) = env::var("CUSTOM_ID_FORMAT") {
            self.custom_id_format = match val.to_lowercase().as_str() {
                "none" => CustomIdFormat::None,
                "uuid" => CustomIdFormat::Uuid,
                "all" => CustomIdFormat::All,
                _ => CustomIdFormat::All,
            }
        }

        if let Ok(val) = env::var("MAX_VALUE_SIZE") {
            if let Ok(max_value_size) = Some(val).unwrap().parse::<usize>() {
                self.max_value_size = max_value_size;
            }
        }
    }
}
