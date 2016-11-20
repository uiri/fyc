use hyper::method::Method;
use hyper::server::{Response, Request, Server};
use hyper::status::StatusCode;
use hyper::uri::RequestUri;

use rustc_serialize::json;
use rustc_serialize::json::Json;
use std::collections::HashMap;
use std::io::Read;

use aci::{ACI, NameValue};
use pod::Pod;

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
    manifest: Json,
    uuid: [u8; 16]
}

pub struct Metadata {
    pod_map: HashMap<[u8; 16], PodMetadata>
}

impl Metadata {
    pub fn new() -> Metadata {
        Metadata {
            pod_map: HashMap::new()
        }
    }

    pub fn register_pod(&mut self, manifest: &str) {
        let pod_metadata = match PodMetadata::new(manifest.to_string()) {
            None => { return; }
            Some(pm) => pm
        };
        self.pod_map.insert(pod_metadata.uuid, pod_metadata);
    }

    #[allow(dead_code)]
    pub fn get_pod(&self, uuid: [u8; 16]) -> String {
        match self.pod_map.get(&uuid) {
            None => String::new(),
            Some(pmd) =>
                match json::encode(pmd) {
                    Err(_) => String::new(),
                    Ok(s) => s
                }
        }
    }

    #[allow(dead_code)]
    pub fn get_app(&self, uuid: [u8; 16], app_name: String) -> String {
        match self.pod_map.get(&uuid) {
            None => String::new(),
            Some(pmd) =>
                match pmd.apps.get(&app_name) {
                    None => String::new(),
                    Some(amd) =>
                        match json::encode(amd) {
                            Err(_) => String::new(),
                            Ok(s) => s
                        }
                }
        }
    }
}

impl PodMetadata {
    pub fn new(manifest: String) -> Option<PodMetadata> {
        let pod : Pod = match json::decode(&manifest) {
            Err(e) => {
                println!("Error decoding manifest json: {}", e);
                return None;
            },
            Ok(a) => a
        };
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
        let manifest_json = match Json::from_str(&manifest) {
            Ok(j) => j,
            Err(_) => Json::Null
        };

        Some(PodMetadata {
            annotations: annotations,
            apps: apps,
            manifest: manifest_json,
            uuid: pod.get_uuid()
        })
    }

    fn sign(&self, mut req: Request, mut res: Response) {
        let ref mut req_body = Vec::new();
        match req.read_to_end(req_body) {
            Err(_) => {
                *res.status_mut() = StatusCode::InternalServerError;
                return;
            }
            _ => {}
        }
        *res.status_mut() = StatusCode::Ok;
        res.send(&req_body[..]).unwrap();
    }

    fn verify(&self, mut req: Request, mut res: Response) {
        let ref mut req_body = Vec::new();
        match req.read_to_end(req_body) {
            Err(_) => {
                *res.status_mut() = StatusCode::InternalServerError;
                return;
            }
            _ => {}
        }
        *res.status_mut() = StatusCode::Ok;
        res.send(&req_body[..]).unwrap();
    }

    fn serve_annotations(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
    }

    fn serve_manifest(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
    }

    fn serve_uuid(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
    }

}

impl AppMetadata {
    fn serve_annotations(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
    }

    fn serve_manifest(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
    }

    fn serve_id(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
    }
}
