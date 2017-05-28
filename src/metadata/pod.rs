use hyper::server::{Response, Request};
use hyper::status::StatusCode;

use std::collections::HashMap;
use std::io::Read;

use rustc_serialize::json;

use pod::Pod;
use util::NameValue;

use super::APP_JSON;
use super::TEXT_PLAIN;
use super::app::AppMetadata;

#[derive(RustcEncodable)]
pub struct PodMetadata {
    annotations: Vec<NameValue>,
    pub apps: HashMap<String, AppMetadata>,
    // hmac: ???
    manifest: String,
    pub uuid: String
}

impl PodMetadata {
    pub fn new(pod: Pod) -> Option<PodMetadata> {
        let annotations = pod.annotations_or_empty();
        let pod_apps = pod.apps_or_empty();
        let mut apps : HashMap<String, AppMetadata> = HashMap::new();
        for a in pod_apps {
            apps.insert(a.get_name(), AppMetadata::new(a.get_annotations(), 
                                                       a.get_app(), 
                                                       a.get_image_id()));
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

    pub fn get_app(&self, app: Option<&str>) -> Option<&AppMetadata> {
        if let Some(app_name) = app {
            return self.apps.get(&String::from(app_name));
        }
        None
    }

    pub fn sign(&self, mut req: Request, mut res: Response) {
        let ref mut req_body = Vec::new();
        if req.read_to_end(req_body).is_err() {
            *res.status_mut() = StatusCode::InternalServerError;
            return;
        }

        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&req_body[..]).unwrap();
    }

    pub fn verify(&self, mut req: Request, mut res: Response) {
        let ref mut req_body = Vec::new();
        if req.read_to_end(req_body).is_err() {
            *res.status_mut() = StatusCode::InternalServerError;
            return;
        }

        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&req_body[..]).unwrap();
    }

    pub fn serve_annotations(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*APP_JSON).clone());
        let send_json = if let Ok(j) = json::encode(&self.annotations) {
            j
        } else {
            String::from("null")
        };
        res.send(&send_json.into_bytes()[..]).unwrap();
    }

    pub fn serve_manifest(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*APP_JSON).clone());
        res.send(&(self.manifest.clone().into_bytes())[..]).unwrap();
    }

    pub fn serve_uuid(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&(self.uuid.clone().into_bytes())[..]).unwrap();
    }

}
