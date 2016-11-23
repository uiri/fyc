//! fyc - Fuck Yo Container
#![feature(process_exec)]

extern crate flate2;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate rustc_serialize;
extern crate tar;
extern crate uuid;

use flate2::read::GzDecoder;

use rustc_serialize::json;

use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::{create_dir, File};
use std::io::Read;
use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::sync::RwLock;
use std::thread;
use std::thread::JoinHandle;

use tar::Archive;

mod aci;
mod metadata;
mod pod;

lazy_static! {
    static ref METADATA_STORE : RwLock<metadata::Metadata> = {
        RwLock::new(metadata::Metadata::new())
    };
}

fn run_aci(arg: String, volumes: &mut HashSet<String>,
           pod_uuid: uuid::Uuid) -> Option<(Sender<bool>, JoinHandle<()>)> {
    let acipath = Path::new(&arg);
    let mut acidirstr = String::from("/opt/fyc/apps/");
    acidirstr.push_str(acipath.file_stem().unwrap().to_str().unwrap());
    acidirstr.push('/');
    let acidir = Path::new(&acidirstr);
    let tarfile = File::open(acipath).unwrap();
    let mut acitar = Archive::new(GzDecoder::new(tarfile).unwrap());
    match create_dir(acidir) {
        Err(_) => {
            println!("Error creating directory for ACI");
            return None;
        },
        _ => {}
    }

    match acitar.unpack(acidir) {
        Err(_) => {
            println!("Error unpacking tarfile");
            return None;
        },
        _ => {}
    }

    let mut manifest_str = String::new();
    match File::open(acidir.join("manifest")).unwrap().read_to_string(&mut manifest_str) {
        Err(_) => {
            println!("Error reading manifest json");
            return None;
        },
        _ => {}
    }

    let manifest : aci::ACI = match json::decode(&manifest_str) {
        Err(e) => {
            println!("Error decoding manifest json: {}", e);
            return None;
        },
        Ok(a) => a
    };

    let mut threadacidirstr = acidirstr.clone();
    threadacidirstr.push_str("rootfs/");
    let mount_points = manifest.mount_volumes("/opt/fyc/volumes/", &threadacidirstr, volumes);
    let (s, r) = channel();
    Some((s, thread::spawn(move || {
        r.recv().unwrap();
        match manifest.exec(&threadacidirstr, pod_uuid) {
            (Some(mut app_child), pre_start, post_stop) => {
                let run_app_child = match pre_start {
                    None => true,
                    Some(mut pre_start_cmd) => {
                        match pre_start_cmd.spawn().unwrap().wait() {
                            Err(e) => {
                                println!("Error in pre-start: {}", e);
                                false
                            }
                            _ => true
                        }
                    }
                };
                if run_app_child {
                    app_child.spawn().unwrap().wait().unwrap();
                }
                match post_stop {
                    None => {},
                    Some(mut post_stop_cmd) => {
                        post_stop_cmd.spawn().unwrap().wait().unwrap();
                    }
                }
            }
            (None, _, _) => {}
        }
        aci::unmount_volumes(mount_points);
    })))
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

    for arg in args {
        match run_aci(arg, &mut volumes, pod_uuid.clone()) {
            None => {},
            Some(t) => { app_threads.push(t); }
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
        match handle.join() {
            Ok(_) => {},
            Err(_) => println!("Oh no, error in a thread.")
        }
    }

    close_service.send(true).unwrap();
}
