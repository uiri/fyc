use hyper::header::ContentType;
use hyper::method::Method;
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use hyper::server::{Response, Request, Server};
use hyper::status::StatusCode;
use hyper::uri::RequestUri;

use serde_json;

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use pod::Pod;

mod app;
mod pod;

use self::pod::PodMetadata;

pub const HOST_PORT: &'static str = "127.0.0.1:2377";
lazy_static! {
    static ref APP_JSON: ContentType = ContentType(Mime(
        TopLevel::Application, SubLevel::Json, vec![]));
    static ref TEXT_PLAIN: ContentType = ContentType(Mime(
        TopLevel::Text, SubLevel::Plain, vec![(
            Attr::Charset, Value::Ext(String::from("us-ascii")))]));
}

pub struct Metadata {
    pod_map: HashMap<String, PodMetadata>
}

pub fn start(md: &'static RwLock<Metadata>) -> Sender<bool> {
    let (s, r) = channel();
    thread::spawn(move || {
        let server = Server::http(HOST_PORT).unwrap();
        let mut listener = server.handle(move |req: Request, res: Response| {
            md.read().unwrap().handle(req, res);
        }).unwrap();
        r.recv().unwrap();
        listener.close().unwrap();
    });
    s
}

impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            pod_map: HashMap::new()
        }
    }

    fn handle(&self, req: Request, mut res: Response) {
        let path_str = match req.uri {
            RequestUri::AbsolutePath(ref p) => p.clone(),
            RequestUri::AbsoluteUri(ref u) => String::from(u.path()),
            _ => {
                *res.status_mut() = StatusCode::BadRequest;
                return;
            }
        };

        let mut req_path_segs = if path_str.starts_with('/') {
            path_str[1..].split('/')
        } else {
            *res.status_mut() = StatusCode::BadRequest;
            return;
        };

        let pmd = if let Some(p) = self.get_by_token(req_path_segs.next()) {
            p
        } else {
            *res.status_mut() = StatusCode::NotFound;
            return;
        };

        if req_path_segs.next() != Some("acMetadata") {
            *res.status_mut() = StatusCode::NotFound;
            return;
        }

        if req_path_segs.next() != Some("v1") {
            *res.status_mut() = StatusCode::NotFound;
            return;
        }

        match req.method {
            Method::Post => {
                if req_path_segs.next() != Some("pod") {
                    *res.status_mut() = StatusCode::NotFound;
                    return;
                }
                if req_path_segs.next() != Some("hmac") {
                    *res.status_mut() = StatusCode::NotFound;
                    return;
                }
                match req_path_segs.next() {
                    Some("sign") => pmd.sign(req, res),
                    Some("verify") => pmd.verify(req, res),
                    _ => {
                        *res.status_mut() = StatusCode::NotFound;
                        return;
                    }
                }
            },
            Method::Get => {
                match req_path_segs.next() {
                    Some("pod") => {
                        match req_path_segs.next() {
                            Some("annotations") =>
                                pmd.serve_annotations(res),
                            Some("manifest") =>
                                pmd.serve_manifest(res),
                            Some("uuid") =>
                                pmd.serve_uuid(res),
                            _ => {
                                *res.status_mut() = StatusCode::NotFound;
                                return;
                            }
                        }
                    }
                    Some("apps") => {
                        let appmd = if let Some(a) = pmd.get_app(req_path_segs.next()) {
                            a
                        } else {
                            *res.status_mut() = StatusCode::NotFound;
                            return;
                        };

                        match req_path_segs.next() {
                            Some("annotations") =>
                                appmd.serve_annotations(res),
                            Some("image") => {
                                match req_path_segs.next() {
                                    Some("manifest") =>
                                        appmd.serve_manifest(res),
                                    Some("id") =>
                                        appmd.serve_id(res),
                                    _ => {
                                        *res.status_mut() =
                                            StatusCode::NotFound;
                                        return;
                                    }
                                }
                            }
                            _ => {
                                *res.status_mut() = StatusCode::NotFound;
                                return;
                            }
                        }
                    }
                    _ => {
                        *res.status_mut() = StatusCode::NotFound;
                        return;
                    }
                }
            },
            _ => *res.status_mut() = StatusCode::MethodNotAllowed
        }
    }

    pub fn register_pod(&mut self, pod: Pod) {
        if let Some(pod_metadata) = PodMetadata::new(pod) {
            self.pod_map.insert(pod_metadata.uuid.clone(), pod_metadata);
        }
    }

    fn get_by_token(&self, token: Option<&str>) -> Option<&PodMetadata> {
        if let Some(tok) = token {
            self.pod_map.get(&String::from(tok))
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_pod(&self, uuid: String) -> String {
        if let Some(pmd) = self.pod_map.get(&uuid) {
            if let Ok(s) = serde_json::to_string(pmd) {
                return s;
            }
        }
        String::new()
    }

    #[allow(dead_code)]
    fn get_app(&self, uuid: String, app_name: String) -> String {
        if let Some(pmd) = self.pod_map.get(&uuid) {
            if let Some(amd) = pmd.apps.get(&app_name) {
                if let Ok(s) = serde_json::to_string(amd) {
                    return s;
                }
            }
        }
        String::new()
    }
}
