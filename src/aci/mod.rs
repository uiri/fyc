use libc;

use std::collections::HashSet;
use std::ffi::CString;
use std::io::Error;
use std::process::Command;

use serde_json;
use uuid::Uuid;
use util::NameValue;

pub mod app;
mod mountpoint;

use self::app::App;
pub use self::mountpoint::MountPoint;
pub use self::app::Isolator;

#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
struct Dependency {
    imageName: String,
    imageID: Option<String>,
    labels: Option<Vec<NameValue>>,
    size: Option<usize>
}

#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
pub struct AciJson {
    acKind: String,
    acVersion: String,
    name: String,
    labels: Option<Vec<NameValue>>,
    app: Option<App>,
    dependencies: Option<Vec<Dependency>>,
    pathWhitelist: Option<Vec<String>>,
    annotations: Option<Vec<NameValue>>
}

pub struct ACI {
    json: AciJson,
    mount_points: Vec<CString>
}

impl ACI {
    pub fn new(manifest_str: &str) -> Result<ACI, Error> {
        let json : AciJson = serde_json::from_str(&manifest_str)?;

        Ok(ACI {
            json: json,
            mount_points: Vec::new()
        })
    }

    pub fn mount_volumes(&mut self, vol_path: &str, app_path: &str, volumes: &mut HashSet<String>) {
        if let Some(ref a) = self.json.app {
            a.mount_volumes(vol_path, app_path, volumes, &mut self.mount_points)
        }
    }

    pub fn unmount_volumes(self) {
        for mount_point in self.mount_points {
            unsafe {
                let e = libc::umount(mount_point.as_ptr());
                if e != 0 {
                    println!("Oh no, could not unmount: {:?}", *libc::__errno_location());
                }
            }
        }
    }

    pub fn exec(&self, dir: &str, pod_uuid: Uuid) -> (Option<Command>, Option<Command>, Option<Command>) {
        let app_name = self.json.name.split('/').last().unwrap();
        match self.json.app {
            None => (None, None, None),
            Some(ref a) => a.exec_app(dir, app_name, pod_uuid)
        }
    }
}
