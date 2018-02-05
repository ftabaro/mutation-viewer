#[macro_use]
extern crate rouille;

use rouille::{Request, Response, start_server};
use rouille::input::json_input;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate flate2;
use flate2::bufread::GzDecoder;

use io::{Result, Error, ErrorKind};
use std::fs::{read_dir, canonicalize, File};
use std::path::{Path, PathBuf};
use std::io;
use std::io::prelude::*;
use std::env::current_dir;

const PORT: &str = "8080";
const URL: &str = "localhost";
const DATA_PATH: &str = "./data/";
const ASSETS_PATH: &str = "./assets/";

struct Dataset {
    name: String,
    vcf: Vec<String>,
}

fn get_datasets(dir: &str) -> Result<Vec<Dataset>> {
    let pbuf_dir = check_absolute_path(dir);
    let mut result = Vec::new();
    for directory in read_dir(pbuf_dir)? {
        let directory = directory?;
        let dir_path = directory.path();
        let mut filenames = Vec::new();
        for file in read_dir(dir_path)? {
            let file = file?;
            let f = file.file_name().into_string();
            match f {
                Ok(filename) => {
                    if filename.ends_with("vcf") | filename.ends_with("vcf.gz") {
                        let chunk = filename.split(".").nth(0);
                        match chunk {
                            None => eprintln!("No file name!"),
                            Some(name) => filenames.push(String::from(name)),
                        }
                    }
                }
                Err(e) => eprintln!("get_datasets -> {:?}", e)
            }
        }
        let d = directory.file_name().into_string();
        match d {
            Ok(dataset_name) => result.push(Dataset { name: dataset_name, vcf: filenames }),
            Err(e) => eprintln!("get_datasets -> {:?}", e)
        }
    }
    Ok(result)
}

fn check_existence(p: &Path) -> () {
    if !p.exists() {
        match File::create(p) {
            Ok(_) => eprintln!("check_existence -> {} created.", p.to_str().unwrap()),
            Err(e) => eprintln!("check_existence -> {:?} - {:?}", p, e)
        }
    } else {
        eprintln!("check_existence -> {} already exists.", p.to_str().unwrap());
    }
    ()
}

fn check_auxiliary_files(root: &String) -> () {
    let mut root_path = check_absolute_path(DATA_PATH);
    root_path.push(root);
    let suffixes = vec![String::from("whitelist.tsv"), String::from("blacklist.tsv")];
    for suffix in suffixes {
        let path = root_path.with_extension(suffix);
        check_existence(&path.as_path());
    }
    ()
}

fn check_absolute_path(d: &str) -> PathBuf {
    let pbuf_dir = PathBuf::from(d);
    if pbuf_dir.is_relative() {
        match current_dir() {
            Ok(mut cwd) => {
                cwd.push(&pbuf_dir);
                match canonicalize(cwd) {
                    Ok(pbuf_dir) => return pbuf_dir,
                    Err(e) => eprintln!("check_absolute_path -> {:?} - {:?}", d, e)
                };
            }
            Err(e) => eprintln!("check_path -> {:?}", e)
        };
    }
    pbuf_dir
}

fn load_file(p: &Path) -> Result<String> {
    if !p.exists() { return Err(Error::new(ErrorKind::NotFound, "file not found.")); }
    let mut content = String::new();
    if p.extension().unwrap() == "gz" {
        let mut file = File::open(p)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let mut gz = GzDecoder::new(&bytes[..]);
        gz.read_to_string(&mut content)?;
    } else {
        let mut file = File::open(p)?;
        file.read_to_string(&mut content)?;
    }
    Ok(content)
}

fn load_asset(filename: &str) -> String {
    let mut p = check_absolute_path(ASSETS_PATH);
    p.push(filename);
    match load_file(p.as_path()) {
        Ok(content) => return content,
        Err(e) => eprintln!("load_asset -> {:?} - {:?}", p, e)
    };
    "".to_string()
}

fn load_data(data_root: &String, ext: &str) -> String {
    let mut p = check_absolute_path(DATA_PATH);
    p.push(data_root);
    p = p.with_extension(ext);
    match load_file(p.as_path()) {
        Ok(content) => return content,
        Err(e) => eprintln!("load_data -> {:?} - {:?}", p, e)
    };
    "".to_string()
}

fn build_index_template(available_datasets: Vec<Dataset>) -> String {
    let mut html = load_asset("index.html");
    let outer_list_template = String::from("<li>{{dataset_name}}<ul>{{vcf_list}}</ul></li>");
    let inner_list_template = String::from("<li><a href=\"/{{dataset_name}}/{{vcf_name}}\">{{vcf_name}}</a></li>");

    let mut outer_list = String::from("");
    for dataset in available_datasets {
        let mut inner_list = String::from("");
        for filename in dataset.vcf {
            let inner_list_element = inner_list_template
                .replace("{{dataset_name}}", &dataset.name)
                .replace("{{vcf_name}}", &filename);
            inner_list.push_str(&inner_list_element);
        }
        let outer_list_element = outer_list_template
            .replace("{{dataset_name}}", &dataset.name)
            .replace("{{vcf_list}}", &inner_list);
        outer_list.push_str(&outer_list_element);
    }
    html = html.replace("{{dataset_list}}", &outer_list);
    html
}

fn build_mutation_template(root: &String) -> String {
    let mut html = load_asset("mutations.html");
    let d3 = load_asset("d3.min.js");
    let xlsx = load_asset("xlsx.core.min.js");
    let vcf = load_data(&root, "vcf.gz");
    let blacklist = load_data(&root, "blacklist.tsv");
    let whitelist = load_data(&root, "whitelist.tsv");

    html = html
        .replace("{{xlsx_lib}}", &xlsx)
        .replace("{{d3_lib}}", &d3)
        .replace("{{vcf_data}}", &vcf)
        .replace("{{blacklist_data}}", &blacklist)
        .replace("{{whitelist_data}}", &whitelist);
    html
}

// HANDLERS

fn handle_index() -> Response {
    let d = get_datasets(DATA_PATH);
    match d {
        Ok(available_datasets) => Response::html(build_index_template(available_datasets)),
        Err(_) => Response::text("Could not list data directory.").with_status_code(500)
    }
}

fn handle_viewer(dataset: String, vcf: String) -> Response {
    let root = format!("{}/{}", dataset, vcf);
    check_auxiliary_files(&root);
    Response::html(build_mutation_template(&root))
}

#[derive(Deserialize, Debug)]
struct Json {
    a: String,
    b: u32
}

fn handle_post(request: &Request) -> Response {
        let json: Json = try_or_400!(json_input(request));
    println!("{:?}", json);
    Response::text("post to update_blacklist")
}

fn main() {
    let addr: String = format!("{}:{}", URL, PORT);
    println!("Server listening at {}", &addr);
    start_server(addr, move |request| {
        router!(request,
                (GET)(/) => { handle_index() },
                (GET)(/{ dataset: String }/{ vcf: String }) => { handle_viewer(dataset, vcf) },
                (POST)(/update_mutation_blacklist) => { handle_post(request) },
                _ => Response::empty_404()
        )
    });
}

