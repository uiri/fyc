use aci;
use aci::Isolator;
use std::collections::HashMap;
use std::collections::HashSet;
use uuid::Uuid;
use util::vec_or_empty;
use util::NameValue;

mod app;
use self::app::App;

#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Volume {
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
#[derive(Serialize, Deserialize)]
pub struct Port {
    name: String,
    hostPort: usize,
    hostIP: Option<String>,
    podPort: Option<aci::app::Port>
}

#[allow(dead_code, non_snake_case)]
#[derive(Serialize, Deserialize)]
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
    pub fn new(uuid: Uuid, version: &str, apps: Option<Vec<App>>,
               volume_set: HashSet<String>, isolators: Option<Vec<Isolator>>,
               annotations: Option<Vec<NameValue>>, ports: Option<Vec<Port>>,
               user_annotations: Option<HashMap<String, String>>,
               user_labels: Option<HashMap<String, String>>) -> Pod {
        let mut volumes : Vec<Volume> = Vec::new();
        for volume in volume_set {
            volumes.push(Volume {
                name: volume,
                kind: String::from("empty"),
                readOnly: Some(false),
                source: Some(String::new()),
                recursive: Some(false),
                mode: None,
                uid: None,
                gid: None
            });
        }
        Pod {
            acKind: String::from("PodManifest"),
            acVersion: String::from(version),
            uuid: uuid.hyphenated().to_string(),
            apps: apps,
            volumes: Some(volumes),
            isolators: isolators,
            annotations: annotations,
            ports: ports,
            userAnnotations: user_annotations,
            userLabels: user_labels
        }
    }

    pub fn annotations_or_empty(&self) -> Vec<NameValue> {
        vec_or_empty(self.annotations.as_ref())
    }

    pub fn apps_or_empty(&self) -> Vec<App> {
        vec_or_empty(self.apps.as_ref())
    }

    pub fn get_uuid(&self) -> String {
        self.uuid.clone()
    }
}
