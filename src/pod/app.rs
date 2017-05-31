use aci::AciJson;
use util::NameValue;
use util::vec_or_empty;
use super::Volume;

#[derive(Clone, Serialize, Deserialize)]
struct AppImage {
    id: String,
    name: Option<String>,
    labels: Option<Vec<NameValue>>
}

#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
struct MountPoint {
    name: String,
    path: String,
    appVolume: Option<Volume>
}

#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
pub struct App {
    name: String,
    image: AppImage,
    app: Option<AciJson>,
    readOnlyRootFS: Option<bool>,
    mounts: Option<Vec<MountPoint>>,
    annotations: Option<Vec<NameValue>>
}

impl App {
    pub fn get_app(&self) -> Option<AciJson> {
        self.app.clone()
    }

    pub fn get_annotations(&self) -> Vec<NameValue> {
        vec_or_empty(self.annotations.as_ref())
    }

    pub fn get_image_id(&self) -> String {
        self.image.id.clone()
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}
