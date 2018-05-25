#[macro_use] extern crate rouille;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate docopt;
extern crate flate2;
extern crate chrono;

use rouille::{Request, Response, start_server};
use rouille::input::json_input;
use docopt::{Docopt, ArgvMap};
use flate2::bufread::GzDecoder;
use chrono::prelude::*;
use io::Result;
use std::fs::{read_dir, canonicalize, File, remove_file};
use std::path::{Path, PathBuf};
use std::io;
use std::io::prelude::*;
use std::io::Read;
use std::env::current_dir;
use std::process::exit;

const USAGE: &str = "
Usage:
    vcfviewer [options] <data_path>

Options:
    --port=N        Port to listen for HTTP requests [default: 8080]
    --address=H     Address to use for listening for HTTP requests [default: localhost]
";

// STRUCTS

struct MyFile {
    name: String,
//    created: DateTime<Local>,
    modified: DateTime<Local>,
    accessed: DateTime<Local>,
}

struct Dataset {
    name: String,
    vcf: Vec<MyFile>,
}

#[derive(Deserialize, Debug)]
struct Json {
    file: String,
    target_list: String,
    signature: String,
    active: bool,
}

//

fn main() {
    let args = parse_args(USAGE);
    let data_path = check_absolute_path(args.get_str("<data_path>"));

    let port: String = args.get_str("--port").parse().unwrap();
    let url: String = args.get_str("--address").parse().unwrap();

    let d3 = include_str!("../assets/d3.min.js");
    let xlsx = include_str!("../assets/xlsx.core.min.js");

    let index_tpl = include_str!("../assets/index.html");
    // For release
    //let mutations_tpl = include_str!("../assets/mutations.html");

    let addr: String = format!("{}:{}", url, port);
    eprintln!("Server listening at {}", &addr);
    start_server(addr, move |request| {
        router!(request,
                (GET)(/) => { handle_index(index_tpl.to_string(), &data_path) },

                (GET)(/{ dataset: String }/{ vcf: String }) => {
                    let d = dataset.replace("%20", " ");
                    let data_root = &mut data_path.join(d);
                    data_root.push(vcf);
                    // For release
                    //handle_viewer(mutations_tpl.to_string(), d3, xlsx, &data_root)

                    // Development
                    let mut file = File::open("/home/betastasis/mviewer/assets/mutations.html").expect("Unable to open mutations.html");;
                    let mut html = String::new();
                    file.read_to_string(&mut html)
                        .expect("something went wrong reading the file");
                    handle_viewer( html, d3, xlsx, &data_root)
                },

                (POST)(/update_mutation_blacklist) => { handle_post(request, &data_path) },
                _ => Response::empty_404()
        )
    });
}

// HANDLERS
fn handle_index(mut tpl: String, data_path: &PathBuf) -> Response {
    let d = get_datasets(data_path);
    match d {
        Ok(available_datasets) => {
            let outer_list_template = String::from("<li class=\"dataset\"><span>{{dataset_name}}</span><ul>{{vcf_list}}</ul></li>");
//            let inner_list_template = String::from("<li><a href=\"/{{dataset_name}}/{{vcf_name}}\" title=\"Created on: {{vcf_creation_date}}\nModified on: {{vcf_mod_data}}\nLast accessed: {{vcf_access_date}}\">{{vcf_name}}</a></li>");
            let inner_list_template = String::from("<li><a href=\"/{{dataset_name}}/{{vcf_name}}\" title=\"Modified on: {{vcf_mod_data}}\nLast accessed: {{vcf_access_date}}\">{{vcf_name}}</a></li>");

            let mut outer_list = String::from("");
            for dataset in available_datasets {
                let mut inner_list = String::from("");
                for file in dataset.vcf {
                    let inner_list_element = inner_list_template
                        .replace("{{dataset_name}}", &dataset.name)
                        .replace("{{vcf_name}}", &file.name)
//                        .replace("{{vcf_creation_date}}", &file.created.format("%a %b %e %T %Y").to_string())
                        .replace("{{vcf_mod_data}}", &file.modified.format("%a %b %e %Y @ %T").to_string())
                        .replace("{{vcf_access_date}}", &file.accessed.format("%a %b %e %Y @ %T").to_string());
                    inner_list.push_str(&inner_list_element);
                }
                let outer_list_element = outer_list_template
                    .replace("{{dataset_name}}", &dataset.name)
                    .replace("{{vcf_list}}", &inner_list);
                outer_list.push_str(&outer_list_element);
            }
            tpl = tpl.replace("{{dataset_list}}", &outer_list);

            Response::html(tpl)
        }
        Err(e) => {
            eprintln!("{:?}", e);
            Response::text("Could not list data directory.").with_status_code(500)
        }
    }
}

fn handle_viewer(mut tpl: String, d3: &str, xlsx: &str, root: &PathBuf) -> Response {

    let vcf = if root.with_extension("vcf.gz").exists() {
        load_data(&root, "vcf.gz")
    } else {
        load_data(&root, "vcf")
    };
    if let Some(vcf) = vcf {
		check_auxiliary_files(&root);
		let blacklist = load_data(&root, "blacklist.tsv").unwrap();
		let whitelist = load_data(&root, "whitelist.tsv").unwrap();
        let samplecolor = load_data(&root, "sample_color.tsv").unwrap();
		tpl = tpl
	        .replace("{{xlsx_lib}}", xlsx)
	        .replace("{{d3_lib}}", d3)
	        .replace("{{vcf_data}}", &vcf)
            .replace("{{samplecolor_data}}", &samplecolor)
	        .replace("{{blacklist_data}}", &blacklist)
	        .replace("{{whitelist_data}}", &whitelist);

		Response::html(tpl)
    } else {
		rouille::Response::empty_404()
	}
}

fn handle_post(request: &Request, data_path: &PathBuf) -> Response {
    let json: Json = try_or_400!(json_input(request));

    let output_file_path = &data_path.join(&json.file.replace("%20", " "));

    let list_file = load_data(output_file_path, &format!("{}.tsv", json.target_list)).unwrap();
    let mut list: Vec<String> = list_file.lines()
        .map(|x| x.split_terminator("\t").collect::<Vec<&str>>().join(":"))
        .collect();

    // json object signature can contain multiple signatures separated by "\n"
    let signatures: Vec<&str> = json.signature.split("\n").collect();
    for sig in signatures {
        // Sample coloring saved with separator ";;" in signature
        let v: Vec<&str> = sig.split(";;").collect();
        let searchable_signature = v[ 0];

        //match &list.iter().position(|x| x == &json.signature) {
        match &list.iter().position(|x| x.starts_with( &searchable_signature)) {
            &Some(idx) => {
                println!("Found {:?} at {:?}", sig, &idx);
                if v.len() == 2 {    
                    // Changing color        
                    list[ idx] = sig.into();
                }
                else if !json.active {
                    list.remove(idx);
                    println!("Removed {}", sig)
                }
            }
            &None => {
                println!("Not Found {:?}", sig);         
                if json.active {
                    println!("Added new {}", sig);
                    list.push( sig.into());
                }
            }
        }
    }

//    PathBuf::from(format!("{}.{}.tsv", json.file, json.target_list)
    match write_file(output_file_path.with_extension(format!("{}.tsv", json.target_list)),
                     list.iter()
                         .map(|x| x.replace(":", "\t"))
                         .collect::<Vec<String>>()
                         .join("\n")) {
        Ok(_) => return Response::text("").with_status_code(200),
        Err(e) => {
            eprintln!("{:?}", e);
            return Response::text("").with_status_code(500);
        }
    };
}


//

fn parse_args(usage: &str) -> ArgvMap {
    match Docopt::new(usage).unwrap().parse() {
        Ok(argv) => return argv,
        Err(_) => {
            eprintln!("{}", &format!("Invalid arguments.\n{}", usage));
            exit(-1)
        }
    }
}

fn get_datasets(dir: &PathBuf) -> Result<Vec<Dataset>> {
//    let pbuf_dir = check_absolute_path(dir);
    let mut result = Vec::new();
	eprintln!("Finding datasets in directory '{:?}'...", dir);
    for directory in read_dir(dir)? {
        let directory = directory?;
        let dir_path = directory.path();
//	eprintln!("dir_path: {:?}", dir_path);
        let mut filenames = Vec::new();
        for file in read_dir(dir_path)? {
//	    eprintln!("file : {:?}", file);
            let file = file?;
            let f = file.file_name().into_string();
            match f {
                Ok(filename) => {
                    if filename.ends_with("vcf") | filename.ends_with("vcf.gz") {
                        let chunk = filename.split(".").nth(0);
                        let metadata = file.metadata()?;
                        match chunk {
                            None => eprintln!("No file name!"),
//                            Some(name) => filenames.push(String::from(name)),
                            Some(name) => filenames.push(MyFile {
                                name: String::from(name),
                                accessed: DateTime::<Local>::from(metadata.accessed()?),
//                                created: DateTime::<Local>::from(metadata.created()?),
                                modified: DateTime::<Local>::from(metadata.modified()?),
                            }),
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

fn check_auxiliary_files(root: &PathBuf) -> () {
    let suffixes = vec![String::from("whitelist.tsv"), String::from("blacklist.tsv"), String::from("sample_color.tsv")];
    for suffix in suffixes {
        let path = root.with_extension(suffix);
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
            Err(e) => {
                eprintln!("check_path -> {:?}", e);
                panic!(e)
            }
        };
    }
    pbuf_dir
}

fn load_file(p: &Path) -> Result<String> {
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

fn load_data(path_to_data: &PathBuf, ext: &str) -> Option<String> {
    let path_to_file = path_to_data.with_extension(ext);
    match load_file(path_to_file.as_path()) {
        Ok(content) => Some(content),
        _ => None
    }
}

fn write_file(file_path: PathBuf, buf: String) -> Result<()> {
    remove_file(file_path.as_path())?;
    let mut file = File::create(file_path.as_path())?;
    file.write_all(buf.as_bytes())?;
    file.flush()?;
    Ok(())
}
