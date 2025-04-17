//! fyc - Fuck Yo Container

extern crate flate2;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tar;
extern crate uuid;

use flate2::read::GzDecoder;

use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::{create_dir, File};
use std::io::{Error, Read};
use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::sync::RwLock;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use tokio::runtime::Handle;

use tar::Archive;

mod aci;
mod metadata;
mod pod;
mod util;

lazy_static! {
    static ref METADATA_STORE : RwLock<metadata::Metadata> = {
        RwLock::new(metadata::Metadata::new())
    };
}

const VOL_DIR : &'static str = "volumes/";
const APP_DIR : &'static str = "apps/";

fn untar(pstr: &String, mut dirstr: String) -> Result<String, Error> {
    let p = Path::new(pstr);
    dirstr.push_str(p.file_stem().unwrap().to_str().unwrap());
    dirstr.push('/');
    let ret = Ok(dirstr.clone());
    let dir = Path::new(&dirstr);
    create_dir(dir)?;
    let file_result = File::open(p);
    if let Ok(opened_file) = file_result {
        let decoder = GzDecoder::new(opened_file);
        Archive::new(decoder).unpack(dir)?;
    } else if let Err(e) = file_result {
        return Err(e);
    }
    ret
}

fn run_aci(volumes: &mut HashSet<String>, pod_uuid: uuid::Uuid,
           mut acidirstr: String, vol_dir: String) -> Result<(Sender<bool>, JoinHandle<()>), Error> {
    let mut manifest_str = String::new();
    File::open(Path::new(&acidirstr).join("manifest"))?.read_to_string(&mut manifest_str)?;
    let mut manifest : aci::ACI = aci::ACI::new(&manifest_str)?;

    acidirstr.push_str("rootfs/");
    let shared_acidir = Arc::new(acidirstr);
    manifest.mount_volumes(&vol_dir, &shared_acidir, volumes);
    let (s, r) = channel();
    Ok((s, thread::spawn({
        let shared_acidir_clone = Arc::clone(&shared_acidir); 
        move || {
            r.recv().unwrap();
            if let (Some(mut app_child), pre_start, post_stop) = manifest.exec(&shared_acidir_clone, pod_uuid) {
                let run_app_child = if let Some(mut pre_start_cmd) = pre_start {
                    if let Err(e) = pre_start_cmd.spawn().unwrap().wait() {
                        println!("Error in pre-start: {}", e);
                        false
                    } else {
                        true
                    }
                } else {
                    true
                };
                if run_app_child {
                    app_child.spawn().unwrap().wait().unwrap();
                }
                if let Some(mut post_stop_cmd) = post_stop {
                    post_stop_cmd.spawn().unwrap().wait().unwrap();
                }
            }
            manifest.unmount_volumes();
    }})))
}

fn main() {
    let mut args = env::args();
    let mut app_threads = Vec::new();
    let mut handles = Vec::new();

    // first argument is the name of the binary
    args.next();

    let mut volumes : HashSet<String> = HashSet::new();

    let close_service = metadata::start(&*METADATA_STORE);

    let pod_uuid = uuid::Uuid::new_v4();
    let pod_version = "0.8.9";
    // METADATA_STORE.write().unwrap().register_pod(format!("{{\"acKind\": \"PodManifest\", \"acVersion\":, \"uuid\": \"{}\", \"annotations\": []}}", pod_uuid));

    let mut pod_dir = String::from("/opt/fyc/");
    pod_dir.push_str(&pod_uuid.hyphenated().to_string());
    pod_dir.push('/');
    if create_dir(pod_dir.clone()).is_err() {
        println!("Error creating directory for Pod");
        return;
    }

    let mut pod_app_dir = pod_dir.clone();
    pod_app_dir.push_str(APP_DIR);
    if create_dir(pod_app_dir.clone()).is_err() {
        println!("Error creating apps directory for Pod");
        return;
    }

    let mut pod_vol_dir = pod_dir.clone();
    pod_vol_dir.push_str(VOL_DIR);
    if create_dir(pod_vol_dir.clone()).is_err() {
        println!("Error creating volumes directory for Pod");
        return;
    }

    for arg in args {
        let untar_str = untar(&arg, pod_app_dir.clone()).unwrap();
        match run_aci(&mut volumes, pod_uuid.clone(), untar_str,
                      pod_vol_dir.clone()) {
            Ok(t) => app_threads.push(t),
            Err(e) => {
                println!("Error running a container: {}", e);
                return;
            }
        }
    }

    let app_pod = pod::Pod::new(
        pod_uuid.clone(), pod_version, Some(Vec::new()), volumes,
        Some(Vec::new()), Some(Vec::new()), Some(Vec::new()),
        Some(HashMap::new()), Some(HashMap::new())
    );

    METADATA_STORE.write().unwrap().register_pod(app_pod);

    for (s, h) in app_threads {
        s.send(true).unwrap();
        handles.push(h);
    }

    for handle in handles {
        if handle.join().is_err() {
            println!("Oh no, error in a thread.")
        }
    }

    Handle::current().block_on(async move {
        close_service.await.send(true).unwrap();
    });
}
