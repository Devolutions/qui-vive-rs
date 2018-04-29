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

use hyper::{Body, StatusCode, Uri, mime};
use hyper::Method::{Get, Post};
use hyper::header::{ContentType, Location};
use hyper::server::{Request, Response, Service};

use futures::Future;
use futures::stream::{Stream};

use rand::{thread_rng, Rng};
use regex::Regex;
use url::{Url};

use std::net::{SocketAddr};

mod config;
use config::QuiViveConfig;

header! { (QuiViveDstUrl, "QuiVive-DstUrl") => [String] }
header! { (QuiViveIdParam, "QuiVive-IdParam") => [String] }

#[derive(Cacheable, Clone, Debug)]
#[cache(expires="86400")] // 24 hours
#[cache(rename="QuiVive")] // use 'QuiVive' prefix
struct QuiViveEntry {
    id: String,
    val: String,
    url: String,
}

pub fn get_timestamp() -> u32 {
    let timespec = time::get_time();
    timespec.sec as u32
}

struct QuiViveService {
    pub cfg: QuiViveConfig,
    pub cache: mouscache::Cache,
}

impl QuiViveService {
    fn gen_id(&self) -> Option<String> {
        let mut rng = thread_rng();
        let id_length = self.cfg.id_length;
        let id_charset = &self.cfg.id_charset.as_ref();
        let id: Option<String> = (0..id_length).map(|_|
            Some(*rng.choose(id_charset)? as char)).collect();
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
            (Get, "/health") => {
                let id = "health".to_string();
                let input = format!("{}", get_timestamp());

                let entry = QuiViveEntry { id: id.clone(), val: input.clone(), url: "".to_string() };
                let cache = self.cache.clone();

                if let Ok(_) = cache.insert(id.clone(), entry.clone()) {
                    match cache.get::<String, QuiViveEntry>(id.clone()) {
                        Ok(Some(ref entry)) if entry.val.eq(&input) => {
                            Box::new(futures::future::ok(Response::new()
                                .with_status(StatusCode::Ok)))
                        }
                        _ => {
                            Box::new(futures::future::ok(Response::new()
                                .with_status(StatusCode::ServiceUnavailable)))
                        }
                    }
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::InternalServerError)))
                }
            }
            (Post, ref x) if RE_KEY.is_match(x) => {
                let id = self.gen_id().unwrap();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let value = String::from_utf8(body.to_vec()).unwrap();

                    let entry = QuiViveEntry { id: id.clone(), val: value, url: "".to_string() };
                    let result = format!("{}/key/{}\n", external_url, id.clone());

                    if let Ok(_) = cache.insert(id.clone(), entry.clone()) {
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

                let cache = self.cache.clone();

                match cache.get::<String, QuiViveEntry>(id.clone()) {
                    Ok(Some(entry)) => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::Ok)
                            .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                            .with_body(entry.val)
                        ))
                    }
                    _ => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::NotFound)))
                    }
                }
            }
            (Post, ref x) if RE_URL.is_match(x) => {
                let id = self.gen_id().unwrap();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let value = String::from_utf8(body.to_vec()).unwrap();

                    let url = value.clone();

                    let entry = QuiViveEntry { id: id.clone(), val: "".to_string(), url: url };
                    let result = format!("{}/{}\n", external_url, id);

                    if let Ok(_) = cache.insert(id.clone(), entry.clone()) {
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

                let cache = self.cache.clone();

                match cache.get::<String, QuiViveEntry>(id.clone()) {
                    Ok(Some(entry)) => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::MovedPermanently)
                            .with_header(Location::new(entry.url))
                        ))
                    }
                    _ => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::NotFound)))
                    }
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
                        let value = String::from_utf8(body.to_vec()).unwrap();

                        let entry = QuiViveEntry { id: id.clone(), val: value, url: url.to_string() };
                        let result = format!("{}/{}\n", external_url, id);

                        if let Ok(_) = cache.insert(id.clone(), entry.clone()) {
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

                let cache = self.cache.clone();

                match cache.get::<String, QuiViveEntry>(id.clone()) {
                    Ok(Some(entry)) => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::MovedPermanently)
                            .with_header(Location::new(entry.url))
                        ))
                    }
                    _ => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::NotFound)))
                    }
                }
            }
            (Get, ref x) if RE_ID.is_match(x) => {
                let cap = RE_ID.captures(x).unwrap();
                let id = cap[1].to_string();

                let cache = self.cache.clone();

                match cache.get::<String, QuiViveEntry>(id.clone()) {
                    Ok(Some(ref entry)) if !entry.url.is_empty() => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::MovedPermanently)
                            .with_header(Location::new(entry.url.clone()))
                        ))
                    }
                    _ => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::NotFound)))
                    }
                }
            }
            _ => {
                Box::new(futures::future::ok(Response::new()
                    .with_status(StatusCode::NotFound)))
            }
        }
    }
}

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
