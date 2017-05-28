use hyper::server::Response;
use hyper::status::StatusCode;

use rustc_serialize::json;

use aci::ACI;
use util::NameValue;

use super::APP_JSON;
use super::TEXT_PLAIN;

#[derive(RustcEncodable)]
pub struct AppMetadata {
    annotations: Vec<NameValue>,
    manifest: Option<ACI>,
    id: String
}

impl AppMetadata {
    pub fn new(annotations: Vec<NameValue>, manifest: Option<ACI>, id: String) -> AppMetadata {
        AppMetadata {
            annotations: annotations,
            manifest: manifest,
            id: id
        }
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
        if let Some(ref m) = self.manifest {
            if let Ok(j) = json::encode(m) {
                res.send(&j.into_bytes()[..]).unwrap();
                return;
            }
        };
        res.send(b"null").unwrap();
    }

    pub fn serve_id(&self, mut res: Response) {
        *res.status_mut() = StatusCode::Ok;
        res.headers_mut().set((*TEXT_PLAIN).clone());
        res.send(&(self.id.clone().into_bytes())[..]).unwrap();
    }
}
