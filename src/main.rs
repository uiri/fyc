//! fyc - Fuck Yo Container

extern crate flate2;
extern crate libc;
extern crate rustc_serialize;
extern crate tar;

use flate2::read::GzDecoder;

// use std::io::prelude;
use std::env;
use std::fs::{create_dir, File};
use std::io::Read;
use std::process::exit;
use std::path::Path;

use rustc_serialize::json;

use tar::Archive;

mod aci;

fn main() {
    let args : Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Please pass a filename as an argument");
        exit(1);
    }
    let acipath = Path::new(&args[1]);
    let mut acidirstr = String::from("/opt/fyc/");
    acidirstr.push_str(acipath.file_stem().unwrap().to_str().unwrap());
    acidirstr.push('/');
    let acidir = Path::new(&acidirstr);
    let tarfile = File::open(acipath).unwrap();
    let mut acitar = Archive::new(GzDecoder::new(tarfile).unwrap());
    match create_dir(acidir) {
        Err(_) => {
            println!("Error creating directory for ACI");
            exit(1);
        },
        _ => {}
    }

    match acitar.unpack(acidir) {
        Err(_) => {
            println!("Error unpacking tarfile");
            exit(1);
        },
        _ => {}
    }

    let mut manifest_str = String::new();
    match File::open(acidir.join("manifest")).unwrap().read_to_string(&mut manifest_str) {
        Err(_) => {
            println!("Error reading manifest json");
            exit(1);
        },
        _ => {}
    }
    let manifest : aci::ACI = match json::decode(&manifest_str) {
        Err(e) => {
            println!("Error decoding manifest json: {}", e);
            exit(1);
        },
        Ok(a) => a
    };

    manifest.exec(acidir.join("rootfs/").to_str().unwrap());
}
