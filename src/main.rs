#[macro_use]
extern crate hyper;

extern crate futures;
extern crate regex;
extern crate rand;
extern crate url;
extern crate time;

#[macro_use]
extern crate clap;

extern crate mouscache;
#[macro_use]
extern crate mouscache_derive;

#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate lazy_static;

use hyper::{Uri};
use std::net::{SocketAddr};

mod config;
use config::QuiViveConfig;

mod service;
use service::QuiViveService;

fn new_cache(ref cfg: &config::QuiViveConfig) -> std::result::Result<mouscache::Cache, mouscache::CacheError> {
    let cache_type = cfg.cache_type.as_ref().map_or("memory", |x| { x.as_str() });
    match cache_type.as_ref() {
        "redis" => {
            let redis_hostname = cfg.redis_hostname.as_ref().map_or("localhost", |x| { x.as_str() });
            let redis_password = cfg.redis_password.as_ref().map(String::as_str);
            mouscache::redis(redis_hostname, redis_password)
        }
        "memory" => {
            Ok(mouscache::memory())
        }
        _ => {
            Err(mouscache::CacheError::Other("invalid cache type".to_string()))
        }
    }
}

fn main() {
    env_logger::init();

    let mut cfg = QuiViveConfig::new();
    cfg.load_cli();
    cfg.load_env();

    let url: Uri = cfg.listener_url.parse().unwrap();
    let address: SocketAddr = url.authority().unwrap().parse().unwrap();

    let new_service = move || {
        let cache = new_cache(&cfg).unwrap();
        Ok(QuiViveService {
            cfg: cfg.clone(),
            cache: cache,
        })
    };

    let server = hyper::server::Http::new()
        .bind(&address, new_service)
        .unwrap();

    info!("running qui-vive at {}", address);
    server.run().unwrap();
}
