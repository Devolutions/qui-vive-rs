
use mouscache;

use hyper;
use hyper::{Body, StatusCode, mime};
use hyper::Method::{Get, Post, Delete};
use hyper::header::{ContentType, Location};
use hyper::server::{Request, Response, Service};

use futures;
use futures::Future;
use futures::stream::{Stream};

use rand::{thread_rng, Rng};
use regex::Regex;
use time;

use uuid::{Uuid};

use url::{Url};

use crate::QuiViveConfig;
use crate::CustomIdFormat;

static NOINDEX: &str = "noindex";
header! { (XRobotsTag, "X-Robots-Tag") => [String] } // noindex

static NOSNIFF: &str = "nosniff";
header! { (XContentTypeOptions, "X-Content-Type-Options") => [String] } // nosniff

header! { (QuiViveDstUrl, "QuiVive-DstUrl") => [String] }
header! { (QuiViveIdParam, "QuiVive-IdParam") => [String] }
header! { (QuiViveExpiration, "QuiVive-Expiration") => [String] }

#[derive(Cacheable, Clone, Debug)]
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

pub struct QuiViveService {
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

    fn get_expiration(&self, request: &Request) -> Option<usize> {
        if let Some(expiration) = request.headers().get::<QuiViveExpiration>() {
            if let Ok(expiration) = expiration.to_string().parse::<u32>() {
                return if expiration == 0 {
                    None
                } else {
                    Some(expiration as usize)
                };
            }
        }
        self.cfg.default_expiration.map(|x| x as usize)
    }
}

impl Service for QuiViveService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response<Body>, Error=hyper::Error>>;

    fn call(&self, request: Request) -> Self::Future {

        lazy_static! {
            static ref RE_ID: Regex = Regex::new(r"^/([\w|-]+)$").unwrap();
            static ref RE_KEY: Regex = Regex::new(r"^/key$").unwrap();
            static ref RE_KEY_ID: Regex = Regex::new(r"^/key/([\w|-]+)$").unwrap();
            static ref RE_URL: Regex = Regex::new(r"^/url$").unwrap();
            static ref RE_URL_ID: Regex = Regex::new(r"^/url/([\w|-]+)$").unwrap();
            static ref RE_INV: Regex = Regex::new(r"^/inv$").unwrap();
            static ref RE_INV_ID: Regex = Regex::new(r"^/inv/([\w|-]+)$").unwrap();
        }

        let method = request.method().clone();
        let path = request.path().clone().to_owned();

        match (method, path.as_str()) {
            (Get, "/health") => {
                let id = "health".to_string();
                let input = format!("{}", get_timestamp());
                let expiration = self.get_expiration(&request);

                let entry = QuiViveEntry { id: id.clone(), val: input.clone(), url: "".to_string() };
                let cache = self.cache.clone();

                if let Ok(_) = cache.insert_with(id.clone(), entry.clone(), expiration) {
                    match cache.get::<String, QuiViveEntry>(id.clone()) {
                        Ok(Some(ref entry)) if entry.val.eq(&input) => {
                            Box::new(futures::future::ok(Response::new()
                                .with_status(StatusCode::Ok)))
                        }
                        _ => {
                            Box::new(futures::future::ok(Response::new()
                                .with_status(StatusCode::InternalServerError)))
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
                let expiration = self.get_expiration(&request);
                let max_value_size = self.cfg.max_value_size;

                Box::new(request.body().concat2().map(move|body| {
                    if body.len() > max_value_size {
                        Response::new()
                            .with_status(StatusCode::PayloadTooLarge)
                    } else if let Ok(value) = String::from_utf8(body.to_vec()) {
                        let entry = QuiViveEntry { id: id.clone(), val: value, url: "".to_string() };
                        let result = format!("{}/key/{}\n", external_url, id.clone());

                        if let Ok(_) = cache.insert_with(id.clone(), entry.clone(), expiration) {
                            Response::new()
                                .with_status(StatusCode::Ok)
                                .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                                .with_header(XContentTypeOptions(NOSNIFF.to_string()))
                                .with_header(XRobotsTag(NOINDEX.to_string()))
                                .with_body(result)
                        } else {
                            Response::new()
                                .with_status(StatusCode::InternalServerError)
                        }
                    } else {
                        Response::new()
                            .with_status(StatusCode::BadRequest)
                    }
                }))
            }
            (Post, ref x) if RE_KEY_ID.is_match(x) => {
                let cap = RE_KEY_ID.captures(x).unwrap();
                let id = cap[1].to_string();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();
                let expiration = self.get_expiration(&request);
                let max_value_size = self.cfg.max_value_size;

                let bad_request = match self.cfg.custom_id_format {
                    CustomIdFormat::None => true,
                    CustomIdFormat::All => false,
                    CustomIdFormat::Uuid => {
                        if let Ok(_) = Uuid::parse_str(id.as_str()) {
                            false
                        } else {
                            true
                        }
                    }
                };

                if bad_request {
                    return Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::BadRequest)));
                }

                Box::new(request.body().concat2().map(move|body| {
                    if body.len() > max_value_size {
                        Response::new()
                            .with_status(StatusCode::PayloadTooLarge)
                    } else if let Ok(value) = String::from_utf8(body.to_vec()) {
                        let entry = QuiViveEntry { id: id.clone(), val: value, url: "".to_string() };
                        let result = format!("{}/key/{}\n", external_url, id.clone());

                        if let Ok(_) = cache.insert_with(id.clone(), entry.clone(), expiration) {
                            Response::new()
                                .with_status(StatusCode::Ok)
                                .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                                .with_header(XContentTypeOptions(NOSNIFF.to_string()))
                                .with_header(XRobotsTag(NOINDEX.to_string()))
                                .with_body(result)
                        } else {
                            Response::new()
                                .with_status(StatusCode::InternalServerError)
                        }
                    } else {
                        Response::new()
                            .with_status(StatusCode::BadRequest)
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
                            .with_header(XContentTypeOptions(NOSNIFF.to_string()))
                            .with_header(XRobotsTag(NOINDEX.to_string()))
                            .with_body(entry.val)
                        ))
                    }
                    _ => {
                        Box::new(futures::future::ok(Response::new()
                            .with_status(StatusCode::NotFound)))
                    }
                }
            }
            (Delete, ref x) if RE_KEY_ID.is_match(x) => {
                let cap = RE_KEY_ID.captures(x).unwrap();
                let id = cap[1].to_string();

                let cache = self.cache.clone();
                let _ = cache.remove::<String, QuiViveEntry>(id.clone());

                // always return 200 OK, even if the resource did not exist (already deleted)
                Box::new(futures::future::ok(Response::new()
                    .with_status(StatusCode::Ok)))
            }
            (Post, ref x) if RE_URL.is_match(x) => {
                let id = self.gen_id().unwrap();
                let cache = self.cache.clone();
                let external_url = self.cfg.external_url.clone();
                let expiration = self.get_expiration(&request);
                let max_value_size = self.cfg.max_value_size;

                Box::new(request.body().concat2().map(move|body| {
                    if body.len() > max_value_size {
                        Response::new()
                            .with_status(StatusCode::PayloadTooLarge)
                    } else if let Ok(value) = String::from_utf8(body.to_vec()) {
                        let url = value.clone();

                        let entry = QuiViveEntry { id: id.clone(), val: "".to_string(), url: url };
                        let result = format!("{}/{}\n", external_url, id);

                        if let Ok(_) = cache.insert_with(id.clone(), entry.clone(), expiration) {
                            Response::new()
                                .with_status(StatusCode::Ok)
                                .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                                .with_header(XContentTypeOptions(NOSNIFF.to_string()))
                                .with_header(XRobotsTag(NOINDEX.to_string()))
                                .with_body(result)
                        } else {
                            Response::new()
                                .with_status(StatusCode::InternalServerError)
                        }
                    } else {
                        Response::new()
                            .with_status(StatusCode::BadRequest)
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
                let expiration = self.get_expiration(&request);
                let max_value_size = self.cfg.max_value_size;

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
                        if body.len() > max_value_size {
                            Response::new()
                                .with_status(StatusCode::PayloadTooLarge)
                        } else if let Ok(value) = String::from_utf8(body.to_vec()) {
                            let entry = QuiViveEntry { id: id.clone(), val: value, url: url.to_string() };
                            let result = format!("{}/{}\n", external_url, id);

                            if let Ok(_) = cache.insert_with(id.clone(), entry.clone(), expiration) {
                                Response::new()
                                    .with_status(StatusCode::Ok)
                                    .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                                    .with_header(XContentTypeOptions(NOSNIFF.to_string()))
                                    .with_header(XRobotsTag(NOINDEX.to_string()))
                                    .with_body(result)
                            } else {
                                Response::new()
                                    .with_status(StatusCode::InternalServerError)
                            }
                        } else {
                            Response::new()
                                .with_status(StatusCode::BadRequest)
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
