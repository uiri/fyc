use aci;
use aci::{ACI, NameValue, Isolator};
use std::collections::HashMap;

#[derive(Clone, RustcDecodable)]
struct AppImage {
    id: String,
    name: Option<String>,
    labels: Option<Vec<NameValue>>
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable)]
struct MountPoint {
    name: String,
    path: String,
    appVolume: Option<Volume>
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable)]
pub struct App {
    name: String,
    image: AppImage,
    app: Option<ACI>,
    readOnlyRootFS: Option<bool>,
    mounts: Option<Vec<MountPoint>>,
    annotations: Option<Vec<NameValue>>
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable)]
struct Volume {
    name: String,
    readOnly: Option<bool>,
    kind: String,
    source: Option<String>,
    recursive: Option<bool>,
    mode: Option<String>,
    uid: Option<String>,
    gid: Option<String>
}

#[allow(dead_code, non_snake_case)]
#[derive(RustcDecodable)]
struct Port {
    name: String,
    hostPort: usize,
    hostIP: Option<String>,
    podPort: Option<aci::Port>
}

#[allow(dead_code, non_snake_case)]
#[derive(RustcDecodable)]
pub struct Pod {
    acVersion: String,
    acKind: String,
    uuid: String,
    apps: Option<Vec<App>>,
    volumes: Option<Vec<Volume>>,
    isolators: Option<Vec<Isolator>>,
    annotations: Option<Vec<NameValue>>,
    ports: Option<Vec<Port>>,
    userAnnotations: Option<HashMap<String, String>>,
    userLabels: Option<HashMap<String, String>>
}

impl Pod {
    pub fn annotations_or_empty(&self) -> Vec<NameValue> {
        match self.annotations {
            None => Vec::new(),
            Some(ref a) => (*a).clone()
        }
    }

    pub fn apps_or_empty(&self) -> Vec<App> {
        match self.apps {
            None => Vec::new(),
            Some(ref a) => (*a).clone()
        }
    }

    pub fn get_uuid(&self) -> String {
        self.uuid.clone()
    }
}

impl App {
    pub fn get_app(&self) -> Option<ACI> {
        self.app.clone()
    }

    pub fn get_annotations(&self) -> Vec<NameValue> {
        match self.annotations {
            None => Vec::new(),
            Some(ref a) => (*a).clone()
        }
    }

    pub fn get_image_id(&self) -> String {
        self.image.id.clone()
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}
