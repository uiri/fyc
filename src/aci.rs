use libc::chroot;

use std::env::set_current_dir;
use std::ffi::CString;
use std::os::unix::process::CommandExt;
use std::process;
use std::process::Command;

static ACE_PATH: &'static str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
static METADATA_URL: &'static str = "http://localhost/";

#[derive(RustcDecodable)]
struct NameValue {
    name: String,
    value: String
}

#[derive(RustcDecodable)]
struct EventHandler {
    exec: Vec<String>,
    name: String
}

#[derive(RustcDecodable)]
struct Isolator {
}

#[allow(dead_code, non_snake_case)]
#[derive(RustcDecodable)]
struct MountPoint {
    name: String,
    path: String,
    readOnly: Option<bool>
}

#[allow(dead_code, non_snake_case)]
#[derive(RustcDecodable)]
struct Port {
    name: String,
    protocol: String,
    port: u16,
    count: u16,
    socketActivated: Option<bool>
}

#[allow(dead_code, non_snake_case)]
#[derive(RustcDecodable)]
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

#[allow(dead_code, non_snake_case)]
#[derive(RustcDecodable)]
struct Dependency {
    imageName: String,
    imageID: Option<String>,
    labels: Option<Vec<NameValue>>,
    size: Option<usize>
}

#[allow(dead_code, non_snake_case)]
#[derive(RustcDecodable)]
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
    fn spawn(&self, exec: &Vec<String>, app_name: &str) -> process::ExitStatus {
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
        let mut child = cmd.spawn().unwrap();
        child.wait().unwrap()
    }

    fn exec_app(&self, dir: &str, app_name: &str) {
        match set_current_dir(dir) {
            Err(e) => {
                println!("chdir failed: {}", e);
                return;
            }
            _ => {}
        }
        let c_dir = CString::new(dir).unwrap();
        unsafe {
            if chroot(c_dir.as_ptr()) != 0 {
                println!("Chroot unsuccessful!");
                return;
            }
        }
        match self.workingDirectory {
            None => {},
            Some(ref wdir) => {
                match set_current_dir(wdir) {
                    Err(e) => {
                        println!("Error performing chdir into specified working directory: {}", e);
                    },
                    _ => {}
                }
            }
        }

        match self.eventHandlers {
            None => {},
            Some(ref ehs) => {
                for eh in ehs {
                    if eh.name == "pre-start" {
                        self.spawn(&eh.exec, app_name);
                    }
                }
            }
        }

        let exit_success = match self.exec {
            None => true,
            Some(ref exec) => self.spawn(exec, app_name).success()
        };

        if exit_success {
            return;
        }
        
        match self.eventHandlers {
            None => {},
            Some(ref ehs) => {
                for eh in ehs {
                    if eh.name == "post-stop" {
                        self.spawn(&eh.exec, app_name);
                    }
                }
            }
        }
    }
}

impl ACI {
    pub fn exec(&self, dir: &str) {
        let app_name = self.name.split('/').last().unwrap();
        match self.app {
            None => {},
            Some(ref a) => a.exec_app(dir, app_name)
        }
    }
}
