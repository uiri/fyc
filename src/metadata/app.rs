use hyper::Response;
use hyper::StatusCode;
use hyper::header::CONTENT_TYPE;
use hyper::header::HeaderValue;

use std::str::FromStr;

use serde_json;

use crate::aci::AciJson;
use crate::util::NameValue;

#[derive(Serialize)]
pub struct AppMetadata {
    annotations: Vec<NameValue>,
    manifest: Option<AciJson>,
    id: String
}

impl AppMetadata {
    pub fn new(annotations: Vec<NameValue>, manifest: Option<AciJson>, id: String) -> AppMetadata {
        AppMetadata {
            annotations: annotations,
            manifest: manifest,
            id: id
        }
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
        if let Some(ref m) = self.manifest {
            if let Ok(j) = serde_json::to_string(m) {
                let ref mut res_body = res.body_mut();
                *res_body = &mut j.clone();
                return res;
            }
        };
        let ref mut res_body = res.body_mut();
        *res_body = &mut String::from_str("null").unwrap();
        res
    }

    pub fn serve_id(&self, mut res: Response<String>) -> Response<String> {
        *res.status_mut() = StatusCode::OK;
        let ref mut res_headers = res.headers_mut();
        res_headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=us-ascii"));
        let ref mut res_body = res.body_mut();
        *res_body = &mut self.id.clone();
        res
    }
}
