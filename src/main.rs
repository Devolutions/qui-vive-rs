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

use hyper::{StatusCode};
use hyper::Method::{Get, Post};
use hyper::server::{Request, Response, Service};

use futures::future::{Future};

use mouscache::{CacheAccess, RedisCache};

use regex::Regex;
use uuid::Uuid;

#[derive(Cacheable, Clone, Debug)]
struct QuiViveEntry {
    token: String,
    key: String,
    value: String,
}

struct QuiVive;

impl Service for QuiVive {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

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

        match (request.method(), request.path()) {
            (&Post, ref x) if RE_NEW_KEY.is_match(x) => {
                let cap = RE_NEW_KEY.captures(x).unwrap();
                let key = cap[1].to_string();
                let token = Uuid::new_v4().to_string();

                Box::new(futures::future::ok(
                    if let Ok(mut cache) = RedisCache::new("localhost", None) {
                        let entry = QuiViveEntry { token: token.clone(), key: key.clone(), value: String::new() };
                        let _ = cache.insert(key.clone(), entry.clone());

                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_body(token)
                    } else {
                        Response::new()
                            .with_status(StatusCode::ServiceUnavailable)
                    }
                ))
            }
            (&Post, ref x) if RE_TOKEN_KEY.is_match(x) => {
                let cap = RE_TOKEN_KEY.captures(x).unwrap();
                let token = cap[1].to_string();
                let key = cap[2].to_string();
                let value = "replace with body value".to_string();

                Box::new(futures::future::ok(
                    if let Ok(mut cache) = RedisCache::new("localhost", None) {
                        let entry = QuiViveEntry { token: token.clone(), key: key.clone(), value: value.clone() };
                        let _ = cache.insert(key.clone(), entry.clone());
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_body(token)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                    }
                ))
            }
            (&Post, ref x) if RE_TOKEN_KEY_VALUE.is_match(x) => {
                let cap = RE_TOKEN_KEY_VALUE.captures(x).unwrap();
                let token = cap[1].to_string();
                let key = cap[2].to_string();
                let value = cap[3].to_string();

                Box::new(futures::future::ok(
                    if let Ok(mut cache) = RedisCache::new("localhost", None) {
                        let entry = QuiViveEntry { token: token.clone(), key: key.clone(), value: value.clone() };
                        let _ = cache.insert(key.clone(), entry.clone());
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_body(token)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                    }
                ))
            }
            (&Get, ref x) if RE_TOKEN_KEY.is_match(x) => {
                let cap = RE_TOKEN_KEY.captures(x).unwrap();
                let token = cap[1].to_string();
                let key = cap[2].to_string();

                Box::new(futures::future::ok(
                    if let Ok(mut cache) = RedisCache::new("localhost", None) {
                        let entry: QuiViveEntry = cache.get(key.clone()).unwrap();
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_body(entry.value)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                            .with_body(token)
                    }
                ))
            }
            (&Get, "/health") => {
                Box::new(futures::future::ok(
                        Response::new()
                            .with_status(StatusCode::Ok)
                ))
            }
            _ => Box::new(futures::future::ok(
                    Response::new().with_status(StatusCode::NotFound),
            )),
        }
    }
}

fn main() {
    env_logger::init();
    let address = "127.0.0.1:8080".parse().unwrap();
    let server = hyper::server::Http::new()
        .bind(&address, || Ok(QuiVive { }))
        .unwrap();
    info!("running qui-vive at {}", address);
    server.run().unwrap();
}
