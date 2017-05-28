#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
pub struct MountPoint {
    pub name: String,
    pub path: String,
    readOnly: Option<bool>
}

impl MountPoint {
    pub fn read_only(&self) -> bool {
        match self.readOnly {
            None => false,
            Some(b) => b
        }
    }
}
