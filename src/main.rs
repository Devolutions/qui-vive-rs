#[macro_use]
extern crate hyper;

extern crate futures;
extern crate regex;
extern crate rand;
extern crate url;

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

use hyper::{Body, StatusCode, mime};
use hyper::Method::{Get, Post};
use hyper::header::{ContentType, Location};
use hyper::server::{Request, Response, Service};

use futures::Future;
use futures::stream::{Stream};

use mouscache::{MemoryCache, RedisCache};

use clap::App;

use rand::{thread_rng, Rng};
use regex::Regex;
use url::{Url};

use std::sync::{Arc, Mutex};
use std::env;

header! { (QuiViveDstUrl, "QuiVive-DstUrl") => [String] }
header! { (QuiViveIdParam, "QuiVive-IdParam") => [String] }
header! { (QuiViveSrcParam, "QuiVive-SrcParam") => [String] }

#[derive(Cacheable, Clone, Debug)]
#[cache(expires="86400")] // 24 hours
struct QuiViveEntry {
    id: String,
    val: String,
    url: String,
}

struct QuiVive {
    pub cfg: QuiViveConfig,
    pub cache: Arc<Mutex<mouscache::Cache>>,
}

fn gen_id() -> Option<String> {
    const CHARSET: &[u8] = b"23456789\
    abcdefghjkimnpqrstuvwxyz\
    ABCDEFGHJKLMNPQRSTUVWXYZ";

    let mut rng = thread_rng();
    let id: Option<String> = (0..9)
        .map(|_| Some(*rng.choose(CHARSET)? as char))
        .collect();
    id
}

impl Service for QuiVive {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response<Body>, Error=hyper::Error>>;

    fn call(&self, request: Request) -> Self::Future {

        lazy_static! {
            static ref RE_ID: Regex = Regex::new(r"^/(\w+)$").unwrap();
            static ref RE_KEY: Regex = Regex::new(r"^/key$").unwrap();
            static ref RE_KEY_ID: Regex = Regex::new(r"^/key/(\w+)$").unwrap();
            static ref RE_URL: Regex = Regex::new(r"^/url$").unwrap();
            static ref RE_URL_ID: Regex = Regex::new(r"^/url/(\w+)$").unwrap();
            static ref RE_INV: Regex = Regex::new(r"^/inv$").unwrap();
            static ref RE_INV_ID: Regex = Regex::new(r"^/inv/(\w+)$").unwrap();
        }

        let method = request.method().clone();
        let path = request.path().clone().to_owned();

        match (method, path.as_str()) {
            (Post, ref x) if RE_KEY.is_match(x) => {
                let id = gen_id().unwrap();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    let entry = QuiViveEntry { id: id.clone(), val: value, url: "".to_string() };
                    let result = format!("{}/key/{}\n", external_url, id);

                    let mut cache_guard = cache.lock().unwrap();
                    if let Ok(_) = cache_guard.insert(id.clone(), entry.clone()) {
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                            .with_body(result)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                    }
                }))
            }
            (Get, ref x) if RE_KEY_ID.is_match(x) => {
                let cap = RE_KEY_ID.captures(x).unwrap();
                let id = cap[1].to_string();

                if let Some(entry) = self.cache.lock().unwrap().get::<String, QuiViveEntry>(id.clone()) {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::Ok)
                        .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                        .with_body(entry.val)
                    ))
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::NotFound)))
                }
            }
            (Post, ref x) if RE_URL.is_match(x) => {
                let id = gen_id().unwrap();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    let url = value.clone();

                    let entry = QuiViveEntry { id: id.clone(), val: "".to_string(), url: url };
                    let result = format!("{}/{}\n", external_url, id);

                    let mut cache_guard = cache.lock().unwrap();
                    if let Ok(_) = cache_guard.insert(id.clone(), entry.clone()) {
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                            .with_body(result)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                    }
                }))
            }
            (Get, ref x) if RE_URL_ID.is_match(x) => {
                let cap = RE_URL_ID.captures(x).unwrap();
                let id = cap[1].to_string();

                if let Some(entry) = self.cache.lock().unwrap().get::<String, QuiViveEntry>(id.clone()) {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::MovedPermanently)
                        .with_header(Location::new(entry.url))
                    ))
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::NotFound)))
                }
            }
            (Post, ref x) if RE_INV.is_match(x) => {
                let id = gen_id().unwrap();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();

                if !request.headers().has::<QuiViveDstUrl>() {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::BadRequest)))
                } else {
                    let dst_url = request.headers().get::<QuiViveDstUrl>().unwrap().to_string();

                    let mut url = Url::parse(&dst_url).unwrap();

                    if let Some(id_param) = request.headers().get::<QuiViveIdParam>() {
                        url.query_pairs_mut().append_pair(id_param.to_string().as_ref(), id.as_ref());
                    }

                    if let Some(src_param) = request.headers().get::<QuiViveSrcParam>() {
                        let src_url = external_url.clone();
                        url.query_pairs_mut().append_pair(src_param.to_string().as_ref(), src_url.as_ref());
                    }

                    Box::new(request.body().concat2().map(move |body| {
                        let mut value = String::from_utf8(body.to_vec()).unwrap();

                        if !value.ends_with('\n') {
                            value.push('\n');
                        }

                        let entry = QuiViveEntry { id: id.clone(), val: value, url: url.to_string() };
                        let result = format!("{}/{}\n", external_url, id);

                        let mut cache_guard = cache.lock().unwrap();
                        if let Ok(_) = cache_guard.insert(id.clone(), entry.clone()) {
                            Response::new()
                                .with_status(StatusCode::Ok)
                                .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                                .with_body(result)
                        } else {
                            Response::new()
                                .with_status(StatusCode::NotFound)
                        }
                    }))
                }
            }
            (Get, ref x) if RE_INV_ID.is_match(x) => {
                let cap = RE_INV_ID.captures(x).unwrap();
                let id = cap[1].to_string();

                if let Some(entry) = self.cache.lock().unwrap().get::<String, QuiViveEntry>(id.clone()) {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::MovedPermanently)
                        .with_header(Location::new(entry.url))
                    ))
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::NotFound)))
                }
            }
            (Get, ref x) if RE_ID.is_match(x) => {
                let cap = RE_ID.captures(x).unwrap();
                let id = cap[1].to_string();

                if let Some(entry) = self.cache.lock().unwrap().get::<String, QuiViveEntry>(id.clone()) {
                    if !entry.url.is_empty() {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::MovedPermanently)
                            .with_header(Location::new(entry.url))
                        ))
                    } else {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::NotFound)))
                    }
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::NotFound)))
                }
            }
            (Get, "/health") => {
                Box::new(futures::future::ok(Response::new()
                    .with_status(StatusCode::Ok)))
            }
            _ => {
                Box::new(futures::future::ok(Response::new()
                    .with_status(StatusCode::NotFound)))
            }
        }
    }
}

#[derive(Clone)]
struct QuiViveConfig {
    external_url: String,
    listener_url: String,
    redis_hostname: Option<String>,
    redis_password: Option<String>,
}

impl QuiViveConfig {

    pub fn new() -> Self {
        QuiViveConfig {
            external_url: "".to_string(),
            listener_url: "".to_string(),
            redis_hostname: None,
            redis_password: None,
        }
    }

    fn load_cli(&mut self) {
        let yaml = load_yaml!("cli.yml");
        let app = App::from_yaml(yaml);
        let matches = app.get_matches();

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
    }

    fn load_env(&mut self) {
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
    }
}

fn main() {
    env_logger::init();

    let mut cfg = QuiViveConfig::new();
    cfg.load_cli();
    cfg.load_env();

    let address = cfg.listener_url.parse().unwrap();

    let new_service = move || {

        let redis_hostname = cfg.redis_hostname.as_ref().map_or("", |x| { x.as_str() });

        let cache = match RedisCache::new(redis_hostname, None) {
            Ok(cache) => cache,
            Err(_) => MemoryCache::new()
        };

        Ok(QuiVive {
            cfg: cfg.clone(),
            cache: Arc::new(Mutex::new(cache))
        })
    };

    let server = hyper::server::Http::new()
        .bind(&address, new_service)
        .unwrap();

    info!("running qui-vive at {}", address);
    server.run().unwrap();
}
