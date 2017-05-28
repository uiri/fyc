use hyper::header::ContentType;
use hyper::method::Method;
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use hyper::server::{Response, Request, Server};
use hyper::status::StatusCode;
use hyper::uri::RequestUri;

use rustc_serialize::json;

use std::collections::HashMap;
use std::io::Read;
use std::sync::RwLock;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use aci::{ACI, NameValue};
use pod::Pod;

pub const HOST_PORT: &'static str = "127.0.0.1:2377";
lazy_static! {
    static ref APP_JSON: ContentType = ContentType(Mime(
        TopLevel::Application, SubLevel::Json, vec![]));
    static ref TEXT_PLAIN: ContentType = ContentType(Mime(
        TopLevel::Text, SubLevel::Plain, vec![(
            Attr::Charset, Value::Ext(String::from("us-ascii")))]));
}

#[derive(RustcEncodable)]
struct AppMetadata {
    annotations: Vec<NameValue>,
    manifest: Option<ACI>,
    id: String
}

#[derive(RustcEncodable)]
struct PodMetadata {
    annotations: Vec<NameValue>,
    apps: HashMap<String, AppMetadata>,
    // hmac: ???
    manifest: String,
    uuid: String
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
            if let Ok(s) = json::encode(pmd) {
                return s;
            }
        }
        String::new()
    }

    #[allow(dead_code)]
    fn get_app(&self, uuid: String, app_name: String) -> String {
        if let Some(pmd) = self.pod_map.get(&uuid) {
            if let Some(amd) = pmd.apps.get(&app_name) {
                if let Ok(s) = json::encode(amd) {
                    return s;
                }
            }
        }
        String::new()
    }
}

impl PodMetadata {
    fn new(pod: Pod) -> Option<PodMetadata> {
        let annotations = pod.annotations_or_empty();
        let pod_apps = pod.apps_or_empty();
        let mut apps : HashMap<String, AppMetadata> = HashMap::new();
        for a in pod_apps {
            apps.insert(a.get_name(), AppMetadata {
                annotations: a.get_annotations(),
                manifest: a.get_app(),
                id: a.get_image_id()
            });
        }

        let manifest_json = if let Ok(j) = json::encode(&pod) {
            j
        } else {
            String::from("")
        };

        Some(PodMetadata {
            annotations: annotations,
            apps: apps,
            manifest: manifest_json,
            uuid: pod.get_uuid()
        })
    }

    fn get_app(&self, app: Option<&str>) -> Option<&AppMetadata> {
        if let Some(app_name) = app {
            return self.apps.get(&String::from(app_name));
        }
        None
    }

    fn sign(&self, mut req: Request, mut res: Response) {
        let ref mut req_body = Vec::new();
        if req.read_to_end(req_body).is_err() {
            *res.status_mut() = StatusCode::InternalServerError;
            return;
        }

        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&req_body[..]).unwrap();
    }

    fn verify(&self, mut req: Request, mut res: Response) {
        let ref mut req_body = Vec::new();
        if req.read_to_end(req_body).is_err() {
            *res.status_mut() = StatusCode::InternalServerError;
            return;
        }

        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&req_body[..]).unwrap();
    }

    fn serve_annotations(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*APP_JSON).clone());
        let send_json = if let Ok(j) = json::encode(&self.annotations) {
            j
        } else {
            String::from("null")
        };
        res.send(&send_json.into_bytes()[..]).unwrap();
    }

    fn serve_manifest(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*APP_JSON).clone());
        res.send(&(self.manifest.clone().into_bytes())[..]).unwrap();
    }

    fn serve_uuid(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&(self.uuid.clone().into_bytes())[..]).unwrap();
    }

}

impl AppMetadata {
    fn serve_annotations(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*APP_JSON).clone());
        let send_json = if let Ok(j) = json::encode(&self.annotations) {
            j
        } else {
            String::from("null")
        };
        res.send(&send_json.into_bytes()[..]).unwrap();
    }

    fn serve_manifest(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*APP_JSON).clone());
        if let Some(ref m) = self.manifest {
            if let Ok(j) = json::encode(m) {
                res.send(&j.into_bytes()[..]).unwrap();
                return;
            }
        };
        res.send(b"null").unwrap();
    }

    fn serve_id(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&(self.id.clone().into_bytes())[..]).unwrap();
    }
}
