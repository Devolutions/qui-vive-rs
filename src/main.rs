extern crate hyper;
extern crate futures;
extern crate regex;
extern crate uuid;
extern crate mouscache;
#[macro_use]
extern crate mouscache_derive;

#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate lazy_static;

use hyper::{Body, StatusCode};
use hyper::Method::{Get, Post};
use hyper::server::{Request, Response, Service};

use futures::Future;
use futures::stream::{Stream};

use mouscache::{CacheAccess, RedisCache};

use regex::Regex;
use uuid::Uuid;

use std::sync::{Arc, Mutex};

#[derive(Cacheable, Clone, Debug)]
struct QuiViveEntry {
    token: String,
    key: String,
    value: String,
}

struct QuiVive {
    pub cache: Arc<Mutex<mouscache::Cache<RedisCache>>>,
}

impl Service for QuiVive {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response<Body>, Error=hyper::Error>>;

    fn call(&self, request: Request) -> Self::Future {

        /**
         * https://github.com/kvaas/docs/blob/master/REST%20API.md
         * POST 	/new/{key}
         * POST 	/{token}/{key}
         * POST 	/{token}/{key}/{value}
         * GET  	/{token}/{key}
        */

        lazy_static! {
            static ref RE_NEW_KEY: Regex = Regex::new(r"^/new/(\w+)$").unwrap();
            static ref RE_TOKEN_KEY: Regex = Regex::new(r"^/(\w+)/(\w+)$").unwrap();
            static ref RE_TOKEN_KEY_VALUE: Regex = Regex::new(r"^/(\w+)/(\w+)/(\w+)$").unwrap();
        }

        let method = request.method().clone();
        let path = request.path().clone().to_owned();

        match (method, path.as_str()) {
            (Post, ref x) if RE_NEW_KEY.is_match(x) => {
                let cap = RE_NEW_KEY.captures(x).unwrap();
                let key = cap[1].to_string();
                let token = Uuid::new_v4().to_string();
                let value = "".to_string();

                let entry = QuiViveEntry { token: token.clone(), key: key.clone(), value: value.clone() };
                if let Ok(_) = self.cache.lock().unwrap().insert(key.clone(), entry.clone()) {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::Ok)
                        .with_body(token)))
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::NotFound)))
                }
            }
            (Post, ref x) if RE_TOKEN_KEY.is_match(x) => {
                let cap = RE_TOKEN_KEY.captures(x).unwrap();
                let token = cap[1].to_string();
                let key = cap[2].to_string();

                let cache = self.cache.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let value = String::from_utf8(body.to_vec()).unwrap();
                    let entry = QuiViveEntry { token: token.clone(), key: key.clone(), value: value };

                    let mut cache_guard = cache.lock().unwrap();
                    if let Ok(_) = cache_guard.insert(key.clone(), entry.clone()) {
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_body(token)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                    }
                }))
            }
            (Post, ref x) if RE_TOKEN_KEY_VALUE.is_match(x) => {
                let cap = RE_TOKEN_KEY_VALUE.captures(x).unwrap();
                let token = cap[1].to_string();
                let key = cap[2].to_string();
                let value = cap[3].to_string();

                let entry = QuiViveEntry { token: token.clone(), key: key.clone(), value: value.clone() };
                if let Ok(_) = self.cache.lock().unwrap().insert(key.clone(), entry.clone()) {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::Ok)
                        .with_body(token)))
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::NotFound)))
                }
            }
            (Get, ref x) if RE_TOKEN_KEY.is_match(x) => {
                let cap = RE_TOKEN_KEY.captures(x).unwrap();
                let key = cap[2].to_string();

                if let Some(entry) = self.cache.lock().unwrap().get::<String, QuiViveEntry>(key.clone()) {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::Ok)
                        .with_body(entry.value)))
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
            Err(_) => unreachable!()
        };
        Ok(QuiVive{ cache: Arc::new(Mutex::new(cache)) } )
    };
    let server = hyper::server::Http::new()
        .bind(&address, new_service)
        .unwrap();
    info!("running qui-vive at {}", address);
    server.run().unwrap();
}
