use libc;

use std::collections::HashSet;
use std::ffi::CString;
use std::process::Command;
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
pub struct ACI {
    acKind: String,
    acVersion: String,
    name: String,
    labels: Option<Vec<NameValue>>,
    app: Option<App>,
    dependencies: Option<Vec<Dependency>>,
    pathWhitelist: Option<Vec<String>>,
    annotations: Option<Vec<NameValue>>
}

pub fn unmount_volumes(mount_points: Vec<CString>) {
    for mount_point in mount_points {
        unsafe {
            let e = libc::umount(mount_point.as_ptr());
            if e != 0 {
                println!("Oh no, could not unmount: {:?}", *libc::__errno_location());
            }
        }
    }
}

impl ACI {
    pub fn mount_volumes(&self, vol_path: &str, app_path: &str, volumes: &mut HashSet<String>) -> Vec<CString> {
        match self.app {
            None => Vec::new(),
            Some(ref a) => a.mount_volumes(vol_path, app_path, volumes)
        }
    }

    pub fn exec(&self, dir: &str, pod_uuid: Uuid) -> (Option<Command>, Option<Command>, Option<Command>) {
        let app_name = self.name.split('/').last().unwrap();
        match self.app {
            None => (None, None, None),
            Some(ref a) => a.exec_app(dir, app_name, pod_uuid)
        }
    }
}
