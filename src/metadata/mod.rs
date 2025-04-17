use hyper::{Response, Request};
use hyper::StatusCode;
use hyper::Method;
use hyper::service::service_fn;
use hyper::body::Incoming;
use hyper_util::server::conn::auto::Builder;
use hyper_util::rt::{TokioExecutor, TokioIo};

use tokio::runtime::Handle;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use serde_json;

use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::mpsc::{channel, Sender};

use crate::pod::Pod;

mod app;
mod pod;

use self::pod::PodMetadata;

pub const HOST_PORT: &'static str = "127.0.0.1:2377";

pub struct Metadata {
    pod_map: HashMap<String, PodMetadata>
}

pub async fn start(md: &'static RwLock<Metadata>) -> Sender<bool> {
    let (s, _r) = channel();
    tokio::spawn(async move {
        let tcp_listener = match TcpListener::bind(HOST_PORT).await {
            Ok(r) => r,
            Err(_e) => { return; }
        };
        let mut join_set = JoinSet::new();
        loop {
            let (stream, _addr) = match tcp_listener.accept().await {
                Ok(x) => x,
                Err(_e) => {
                    continue;
                }
            };
            
            let serve_connection = async move {
                let _r = Builder::new(TokioExecutor::new())
                    .serve_connection(TokioIo::new(stream), service_fn(|req: Request<Incoming>| async move {
                        let metadata = md.read().unwrap();
                        let res = metadata.handle(req);
                        Ok::<Response<String>, String>(res)
                    }));
            };
            
            join_set.spawn(serve_connection);
        }

        // TODO: Graceful
        // while let Some(_) = join_set.join_next().await {}

        //let server = Builder::http1() // Server::http(HOST_PORT).unwrap();
        // r.recv().unwrap();
        // listener.close().unwrap();
    });
    s
}

impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            pod_map: HashMap::new()
        }
    }

    fn handle(&self, req: Request<Incoming>) -> Response<String> {
        let mut res: Response<String> = Default::default();
        let path_str = req.uri().path();
        let handle = Handle::current();

        let mut req_path_segs = if path_str.starts_with('/') {
            path_str[1..].split('/')
        } else {
            *res.status_mut() = StatusCode::BAD_REQUEST;
            return res;
        };

        let pmd = if let Some(p) = self.get_by_token(req_path_segs.next()) {
            p
        } else {
            *res.status_mut() = StatusCode::NOT_FOUND;
            return res;
        };

        if req_path_segs.next() != Some("acMetadata") {
            *res.status_mut() = StatusCode::NOT_FOUND;
            return res;
        }

        if req_path_segs.next() != Some("v1") {
            *res.status_mut() = StatusCode::NOT_FOUND;
            return res;
        }

        match *req.method() {
            Method::POST => {
                if req_path_segs.next() != Some("pod") {
                    *res.status_mut() = StatusCode::NOT_FOUND;
                    return res;
                }
                if req_path_segs.next() != Some("hmac") {
                    *res.status_mut() = StatusCode::NOT_FOUND;
                    return res;
                }
                match req_path_segs.next() {
                    Some("sign") => handle.block_on(async move { pmd.sign(req, res).await }),
                    Some("verify") => handle.block_on(async move { pmd.verify(req, res).await }),
                    _ => {
                        *res.status_mut() = StatusCode::NOT_FOUND; res
                    }
                }
            },
            Method::GET => {
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
                                *res.status_mut() = StatusCode::NOT_FOUND; res
                            }
                        }
                    }
                    Some("apps") => {
                        let appmd = if let Some(a) = pmd.get_app(req_path_segs.next()) {
                            a
                        } else {
                            *res.status_mut() = StatusCode::NOT_FOUND;
                            return res;
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
                                            StatusCode::NOT_FOUND;
                                        res
                                    }
                                }
                            }
                            _ => {
                                *res.status_mut() = StatusCode::NOT_FOUND;
                                return res;
                            }
                        }
                    }
                    _ => {
                        *res.status_mut() = StatusCode::NOT_FOUND;
                        return res;
                    }
                }
            },
            _ => { *res.status_mut() = StatusCode::METHOD_NOT_ALLOWED; res }
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
