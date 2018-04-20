#[macro_use]
extern crate hyper;

extern crate futures;
extern crate regex;
extern crate rand;

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

use rand::{thread_rng, Rng};
use regex::Regex;

use std::sync::{Arc, Mutex};

header! { (Destination, "Destination") => [String] }

#[derive(Cacheable, Clone, Debug)]
struct QuiViveEntry {
    id: String,
    val: String,
    url: String,
}

struct QuiVive {
    pub url_prefix: String,
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
                let url_prefix = self.url_prefix.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    let entry = QuiViveEntry { id: id.clone(), val: value, url: "".to_string() };
                    let result = format!("{}/key/{}\n", url_prefix, id);

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
                let url_prefix = self.url_prefix.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    let url = value.clone();

                    let entry = QuiViveEntry { id: id.clone(), val: "".to_string(), url: url };
                    let result = format!("{}/{}\n", url_prefix, id);

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
                let url_prefix = self.url_prefix.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    let url = value.clone();

                    let entry = QuiViveEntry { id: id.clone(), val: value, url: url };
                    let result = format!("{}/{}\n", url_prefix, id);

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
    let address = "127.0.0.1:8080".parse().unwrap();
    let new_service = || {
        let cache = match RedisCache::new("localhost", None) {
            Ok(cache) => cache,
            Err(_) => MemoryCache::new()
        };
        Ok(QuiVive {
            url_prefix: "127.0.0.1:8080".to_string(),
            cache: Arc::new(Mutex::new(cache))
        })
    };
    let server = hyper::server::Http::new()
        .bind(&address, new_service)
        .unwrap();
    info!("running qui-vive at {}", address);
    server.run().unwrap();
}
