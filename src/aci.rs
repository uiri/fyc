use libc::{chroot, mount, MS_BIND, MS_RDONLY};
use libc;

use std::collections::HashSet;
use std::env::set_current_dir;
use std::ffi::CString;
use std::fs::create_dir;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::ptr;

static ACE_PATH: &'static str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
static METADATA_URL: &'static str = "http://localhost/";

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct NameValue {
    name: String,
    value: String
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
struct EventHandler {
    exec: Vec<String>,
    name: String
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct Isolator {
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable, RustcEncodable)]
struct MountPoint {
    name: String,
    path: String,
    readOnly: Option<bool>
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct Port {
    name: String,
    protocol: String,
    port: u16,
    count: u16,
    socketActivated: Option<bool>
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable, RustcEncodable)]
struct App {
    exec: Option<Vec<String>>,
    user: String,
    group: String,
    supplementaryGIDs: Option<Vec<usize>>,
    eventHandlers: Option<Vec<EventHandler>>,
    workingDirectory: Option<String>,
    environment: Option<Vec<NameValue>>,
    isolators: Option<Vec<Isolator>>,
    mountPoints: Option<Vec<MountPoint>>,
    ports: Option<Vec<Port>>
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable, RustcEncodable)]
struct Dependency {
    imageName: String,
    imageID: Option<String>,
    labels: Option<Vec<NameValue>>,
    size: Option<usize>
}

#[allow(non_snake_case)]
#[derive(Clone, RustcDecodable, RustcEncodable)]
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

impl App {
    fn prep_cmd(&self, exec: &Vec<String>, dir: &str, app_name: &str) -> Command {
        let mut cmd = Command::new(&exec[0]);
        cmd.args(&exec[1..]);
        match self.user.parse::<u32>() {
            Err(_) => {}, // find actual user
            Ok(userid) => { cmd.uid(userid); }
        }
        match self.group.parse::<u32>() {
            Err(_) => {}, // find actual group
            Ok(groupid) => { cmd.gid(groupid); }
        }
        cmd.env("PATH", ACE_PATH);
        cmd.env("AC_APP_NAME", app_name);
        cmd.env("AC_METADATA_URL", METADATA_URL);
        match self.environment {
            None => {},
            Some(ref env_vars) => {
                for ekv in env_vars {
                    cmd.env(&ekv.name, &ekv.value);
                }
            }
        }
        let closed_dir = String::from(dir);
        let work_dir = match self.workingDirectory {
            None => None,
            Some(ref wdir) => Some(wdir.clone())
        };

        cmd.before_exec(move || {
            match set_current_dir(&closed_dir) {
                Err(e) => {
                    println!("chdir failed: {}", e);
                    return Err(e);
                }
                _ => {}
            }

            let c_dir = CString::new(closed_dir.clone()).unwrap();
            unsafe {
                let e = chroot(c_dir.as_ptr());
                if e != 0 {
                    println!("Chroot unsuccessful!");
                    return Err(io::Error::last_os_error());
                }
            }

            match work_dir {
                None => {},
                Some(ref wdir) => {
                    match set_current_dir(wdir) {
                        Err(e) => {
                            println!("chdir failed: {}", e);
                            return Err(e);
                        }
                        _ => {}
                    }
                }
            }

            Ok(())
        });
        cmd
    }

    fn mount_points_or_empty(&self) -> Vec<MountPoint> {
        match self.mountPoints {
            Some(ref v) => (*v).clone(),
            None => Vec::new()
        }
    }

    pub fn mount_volumes(&self, vol_path: &str, app_path: &str, volumes: &mut HashSet<String>) {
        for mount_point in self.mount_points_or_empty() {
            let mut mount_src_str = String::from(vol_path);
            mount_src_str.push_str(&mount_point.name);

            if !volumes.contains(&mount_point.name) {
                match create_dir(Path::new(&mount_src_str)) {
                    Err(_) => {
                        println!("Error creating directory for volume! Oh no!");
                        return;
                    }
                    _ => {}
                }
                volumes.insert(mount_point.name.clone());
            }

            let mount_src = CString::new(mount_src_str).unwrap();

            let mut mount_dst_str = String::from(app_path);
            mount_dst_str.push_str(&mount_point.path);
            match create_dir(Path::new(&mount_dst_str)) {
                Err(_) => {
                    println!("Error creating directory for volume! Oh no!");
                    return;
                }
                _ => {}
            }
            let mount_dst = CString::new(mount_dst_str).unwrap();

            let mount_flags = if mount_point.read_only() {
                MS_BIND & MS_RDONLY
            } else {
                MS_BIND
            };

            unsafe {
                let e = mount(mount_src.as_ptr(), mount_dst.as_ptr(), ptr::null(), mount_flags, ptr::null());
                if e != 0 {
                    println!("Oh no, could not mount a volume: {:?}", *libc::__errno_location());
                }
            }
        }
    }

    fn find_event_handle(&self, ehs: &Vec<EventHandler>, dir: &str, app_name: &str, event_name: &str) -> Option<Command> {
        for eh in ehs {
            if eh.name == event_name {
                return Some(self.prep_cmd(&eh.exec, dir, app_name));
            }
        }
        return None;
    }

    fn exec_app(&self, dir: &str, app_name: &str) -> (Option<Command>, Option<Command>, Option<Command>) {
        let app_child = match self.exec {
            None => { return (None, None, None); }
            Some(ref exec) => self.prep_cmd(exec, dir, app_name)
        };

        let pre_start = match self.eventHandlers {
            None => None,
            Some(ref ehs) => self.find_event_handle(ehs, dir, app_name, "pre-start")
        };


        let post_stop = match self.eventHandlers {
            None => None,
            Some(ref ehs) => self.find_event_handle(ehs, dir, app_name, "post-stop")
        };

        (Some(app_child), pre_start, post_stop)
    }
}

impl ACI {
    pub fn mount_volumes(&self, vol_path: &str, app_path: &str, volumes: &mut HashSet<String>) {
        match self.app {
            None => {},
            Some(ref a) => a.mount_volumes(vol_path, app_path, volumes)
        }
    }

    pub fn exec(&self, dir: &str) -> (Option<Command>, Option<Command>, Option<Command>) {
        let app_name = self.name.split('/').last().unwrap();
        match self.app {
            None => (None, None, None),
            Some(ref a) => a.exec_app(dir, app_name)
        }
    }
}

impl MountPoint {
    pub fn read_only(&self) -> bool {
        match self.readOnly {
            None => false,
            Some(b) => b
        }
    }
}
