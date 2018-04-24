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

use hyper::{Body, StatusCode, Uri, mime};
use hyper::Method::{Get, Post};
use hyper::header::{ContentType, Location};
use hyper::server::{Request, Response, Service};

use futures::Future;
use futures::stream::{Stream};

use mouscache::{MemoryCache, RedisCache};

use rand::{thread_rng, Rng};
use regex::Regex;
use url::{Url};

use std::sync::{Arc, Mutex};
use std::net::{SocketAddr};

mod config;
use config::QuiViveConfig;

header! { (QuiViveDstUrl, "QuiVive-DstUrl") => [String] }
header! { (QuiViveIdParam, "QuiVive-IdParam") => [String] }

#[derive(Cacheable, Clone, Debug)]
#[cache(expires="86400")] // 24 hours
struct QuiViveEntry {
    id: String,
    val: String,
    url: String,
}

struct QuiViveService {
    pub cfg: QuiViveConfig,
    pub cache: Arc<Mutex<mouscache::Cache>>,
}

impl QuiViveService {
    fn gen_id(&self) -> Option<String> {
        const CHARSET: &[u8] = b"23456789\
            abcdefghjkimnpqrstuvwxyz\
            ABCDEFGHJKLMNPQRSTUVWXYZ";

        let mut rng = thread_rng();
        let id: Option<String> = (0..9)
            .map(|_| Some(*rng.choose(CHARSET)? as char))
            .collect();
        id
    }
}

impl Service for QuiViveService {
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
                let id = self.gen_id().unwrap();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    debug!("key: {} value: {}", id, value);

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
                let id = self.gen_id().unwrap();
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
                let id = self.gen_id().unwrap();
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

fn main() {
    env_logger::init();

    let mut cfg = QuiViveConfig::new();
    cfg.load_cli();
    cfg.load_env();

    let url: Uri = cfg.listener_url.parse().unwrap();
    let address: SocketAddr = url.authority().unwrap().parse().unwrap();

    let new_service = move || {

        let redis_hostname = cfg.redis_hostname.as_ref().map_or("localhost", |x| { x.as_str() });
        let redis_password = cfg.redis_password.as_ref().map(String::as_str);

        let cache = match RedisCache::new(redis_hostname, redis_password) {
            Ok(cache) => cache,
            Err(_) => MemoryCache::new()
        };

        Ok(QuiViveService {
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
