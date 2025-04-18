use hyper::{Response, Request};
use hyper::StatusCode;
use hyper::header::CONTENT_TYPE;
use hyper::header::HeaderValue;
use hyper::body::{Incoming, Bytes};

use http_body_util::BodyExt;
use futures_util::StreamExt;
use futures_util::FutureExt;

use std::convert::Infallible;
use std::collections::HashMap;

use serde_json;

use crate::pod::Pod;
use crate::util::NameValue;

use super::app::AppMetadata;

#[derive(Serialize)]
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

        let manifest_json = if let Ok(j) = serde_json::to_string(&pod) {
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

    pub async fn sign(&self, mut req: Request<Incoming>, mut res: Response<String>) -> Response<String> {
        let req_body: &mut Incoming = req.body_mut();
        let mut req_stream = req_body.frame().map(|f| Ok::<Bytes, Infallible>(f.unwrap().unwrap().into_data().unwrap())).into_stream();
        let mut req_bodystr = String::new();
        while let Some(req_data) = req_stream.next().await {
            req_bodystr.push_str(&String::from_utf8(req_data.unwrap().to_vec()).unwrap());
        }
        *res.status_mut() = StatusCode::OK;
        let ref mut res_headers = res.headers_mut();
        res_headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=us-ascii"));
        let ref mut res_body = res.body_mut();
        *res_body = &mut req_bodystr;
        res
    }

    pub async fn verify(&self, mut req: Request<Incoming>, mut res: Response<String>) -> Response<String> {
        let req_body: &mut Incoming = req.body_mut();
        let mut req_stream = req_body.frame().map(|f| Ok::<Bytes, Infallible>(f.unwrap().unwrap().into_data().unwrap())).into_stream();
        let mut req_bodystr = String::new();
        while let Some(req_data) = req_stream.next().await {
            req_bodystr.push_str(&String::from_utf8(req_data.unwrap().to_vec()).unwrap());
        }

        *res.status_mut() = StatusCode::OK;
        let ref mut res_headers = res.headers_mut();
        res_headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=us-ascii"));
        let ref mut res_body = res.body_mut();
        *res_body = &mut req_bodystr;
        res
    }

    pub fn serve_annotations(&self, mut res: Response<String>) -> Response<String> {
        *res.status_mut() = StatusCode::OK;
        let ref mut res_headers = res.headers_mut();
        res_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let send_json = if let Ok(j) = serde_json::to_string(&self.annotations) {
            j
        } else {
            String::from("null")
        };
        let ref mut res_body = res.body_mut();
        *res_body = &mut send_json.clone();
        res
    }

    pub fn serve_manifest(&self, mut res: Response<String>) -> Response<String> {
        *res.status_mut() = StatusCode::OK;
        let ref mut res_headers = res.headers_mut();
        res_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let ref mut res_body = res.body_mut();
        *res_body = &mut self.manifest.clone();
        res
    }

    pub fn serve_uuid(&self, mut res: Response<String>) -> Response<String> {
        *res.status_mut() = StatusCode::OK;
        res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=us-ascii"));
        let ref mut res_body = res.body_mut();
        *res_body = &mut self.uuid.clone();
        res
    }

}
