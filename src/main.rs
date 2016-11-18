//! fyc - Fuck Yo Container
#![feature(process_exec)]

extern crate flate2;
extern crate libc;
extern crate rustc_serialize;
extern crate tar;

use flate2::read::GzDecoder;

use rustc_serialize::json;

use std::env;
use std::fs::{create_dir, File};
use std::io::Read;
use std::process::exit;
use std::path::Path;
use std::thread;
use std::thread::JoinHandle;

use tar::Archive;

mod aci;
mod metadata;
mod pod;

fn run_aci(arg: String) -> Option<JoinHandle<()>> {
    let acipath = Path::new(&arg);
    let mut acidirstr = String::from("/opt/fyc/");
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
    Some(thread::spawn(move || {
        threadacidirstr.push_str("rootfs/");
        match manifest.exec(&threadacidirstr){
            (Some(mut app_child), pre_start, post_stop) => {
                match pre_start {
                    None => {},
                    Some(mut pre_start_cmd) => {
                        match pre_start_cmd.spawn().unwrap().wait() {
                            Err(e) => {
                                println!("Error in pre-start: {}", e);
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                let exit = app_child.spawn().unwrap().wait().unwrap();
                match post_stop {
                    None => {},
                    Some(mut post_stop_cmd) => {
                        if !exit.success() {
                            post_stop_cmd.spawn().unwrap();
                        }
                    }
                }
            }
            (None, _, _) => {}
        }
    }))
}

fn main() {
    let mut args = env::args();
    let mut handles = Vec::new();

    // first argument is the name of the binary
    args.next();

    for arg in args {
        match run_aci(arg) {
            None => {},
            Some(h) => { handles.push(h); }
        }
    }

    for handle in handles {
        match handle.join() {
            Ok(_) => {},
            Err(_) => println!("Oh no, error in a thread.")
        }
    }

    let mut metadata_store = metadata::Metadata::new();
    metadata_store.register_pod("{\"uuid\": \"\"}");
}
