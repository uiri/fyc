use libc::{chroot, mount, MS_BIND, MS_RDONLY, MS_NODEV, MS_NOEXEC, MS_NOSUID};
use libc;

use crate::metadata;

use std::collections::HashSet;
use std::env::set_current_dir;
use std::ffi::CString;
use std::fs::create_dir;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::ptr;
use uuid::Uuid;
use crate::util::vec_or_empty;
use crate::util::NameValue;

use super::MountPoint;

const ACE_PATH: &'static str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
const FYC: &'static str = "fyc";

#[derive(Clone, Serialize, Deserialize)]
struct EventHandler {
    exec: Vec<String>,
    name: String
}

#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Port {
    name: String,
    protocol: String,
    port: u16,
    count: u16,
    socketActivated: Option<bool>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Isolator {
}

#[allow(non_snake_case)]
#[derive(Clone, Serialize, Deserialize)]
pub struct App {
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

fn mount_system_volumes(app_path: &str, mount_points: &mut Vec<CString>) {
    let system_volumes = vec!["proc", "sys", "dev"];

    for system_volume in system_volumes {
        let mut mount_dir = String::from(app_path);
        mount_dir.push_str(system_volume);
        mount_dir.push('/');

        let mount_dst = CString::new(mount_dir.clone()).unwrap();

        let mut mount_src_str = String::from("/");
        mount_src_str.push_str(system_volume);
        let mount_src = CString::new("/proc").unwrap();

        let mount_flags = MS_BIND | MS_NODEV | MS_NOSUID | MS_NOEXEC;

        if let Err(e) = create_dir(Path::new(&mount_dir)) {
            println!("Error creating directory for volume! Oh no: {}", e);
            return;
        }

        unsafe {
            let e = mount(mount_src.as_ptr(), mount_dst.as_ptr(),
                          ptr::null(), mount_flags, ptr::null());
            if e != 0 {
                println!("Oh no, could not mount a volume: {:?}",
                         *libc::__errno_location());
            }
        }
        mount_points.push(mount_dst);
    }
}

impl App {
    fn prep_cmd(&self, exec: &Vec<String>, dir: &str,
                app_name: &str, pod_uuid: Uuid) -> Command {
        let mut cmd = Command::new(&exec[0]);
        cmd.args(&exec[1..]);
        if let Ok(userid) = self.user.parse::<u32>() {
            cmd.uid(userid);
        }

        if let Ok(groupid) = self.group.parse::<u32>() {
            cmd.gid(groupid);
        }

        let mut metadata_url = String::from("http://");
        metadata_url.push_str(metadata::HOST_PORT);
        metadata_url.push('/');
        metadata_url.push_str(&pod_uuid.hyphenated().to_string());

        cmd.env("PATH", ACE_PATH);
        cmd.env("AC_APP_NAME", app_name);
        cmd.env("AC_METADATA_URL", metadata_url);
        cmd.env("container", FYC);
        if let Some(ref env_vars) = self.environment {
            for ekv in env_vars {
                cmd.env(&ekv.name, &ekv.value);
            }
        }

        let closed_dir = String::from(dir);
        let work_dir = match self.workingDirectory {
            None => None,
            Some(ref wdir) => Some(wdir.clone())
        };

        unsafe {
            cmd.pre_exec(move || {
                match set_current_dir(&closed_dir) {
                    Err(e) => {
                        println!("chdir failed: {}", e);
                        return Err(e);
                    }
                    _ => {}
                }

                let c_dir = CString::new(closed_dir.clone()).unwrap();

                let e = chroot(c_dir.as_ptr());
                if e != 0 {
                    println!("Chroot unsuccessful!");
                    return Err(io::Error::last_os_error());
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
        }
        cmd
    }

    fn mount_points_or_empty(&self) -> Vec<MountPoint> {
        vec_or_empty(self.mountPoints.as_ref())
    }

    pub fn mount_volumes(&self, vol_path: &str, app_path: &str,
                         volumes: &mut HashSet<String>,
                         mount_points: &mut Vec<CString>) {
        mount_system_volumes(app_path, mount_points);

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
                let e = mount(mount_src.as_ptr(), mount_dst.as_ptr(),
                              ptr::null(), mount_flags, ptr::null());
                if e != 0 {
                    println!("Oh no, could not mount a volume: {:?}",
                             *libc::__errno_location());
                }
            }
            mount_points.push(mount_dst);
        }
    }

    fn find_event_handle(&self, ehs: &Vec<EventHandler>, dir: &str,
                         app_name: &str, pod_uuid: Uuid,
                         event_name: &str) -> Option<Command> {
        for eh in ehs {
            if eh.name == event_name {
                return Some(self.prep_cmd(&eh.exec, dir, app_name, pod_uuid));
            }
        }
        return None;
    }

    pub fn exec_app(&self, dir: &str, app_name: &str,
                pod_uuid: Uuid) -> (Option<Command>, Option<Command>,
                                    Option<Command>) {
        let app_child = if let Some(ref exec) = self.exec {
            self.prep_cmd(exec, dir, app_name, pod_uuid)
        } else {
            return (None, None, None);
        };

        let pre_start = if let Some(ref ehs) = self.eventHandlers {
            self.find_event_handle(ehs, dir, app_name, pod_uuid, "pre-start")
        } else {
            None
        };

        let post_stop = if let Some(ref ehs) = self.eventHandlers {
            self.find_event_handle(ehs, dir, app_name, pod_uuid, "post-stop")
        } else {
            None
        };

        (Some(app_child), pre_start, post_stop)
    }
}
