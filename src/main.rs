#[macro_use]
extern crate rouille;

use rouille::{Response, start_server};

use io::Result;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use std::io;

const PORT: &str = "8080";
const URL: &str = "localhost";
const DATA_PATH: &'static str = "/home/ftabaro/IdeaProjects/RustyVCF/data";

struct Dataset {
    name: String,
    vcf: Vec<String>,
}

fn get_datasets(dir: &str) -> Result<Vec<Dataset>> {
    let mut result = Vec::new();
    for directory in read_dir(Path::new(dir))? {
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
                            None => panic!("No file name!"),
                            Some(name) => filenames.push(String::from(name)),
                        }
                    }
                }
                Err(e) => panic!(e)
            }
        }
        let d = directory.file_name().into_string();
        match d {
            Ok(dataset_name) => result.push(Dataset { name: dataset_name, vcf: filenames }),
            Err(e) => panic!(e)
        }
    }
    Ok(result)
}

fn build_index_template(available_datasets: Vec<Dataset>) -> String {
    let mut html = String::from(include_str!("/home/ftabaro/IdeaProjects/RustyVCF/assets/index.html"));
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

fn check_existance(p: &Path) -> () {
    if !p.is_file() {
        match File::create(p) {
            Ok(_) => eprintln!("{} created.", p.to_str().unwrap()),
            Err(e) => panic!(e)
        }
    } else {
        eprintln!("{} already exists.", p.to_str().unwrap());
    }
    ()
}

fn check_auxiliary_files(root: &String) -> () {
    let root_path = PathBuf::from(format!("{}/{}", DATA_PATH, root));
    let suffixes = vec![String::from("whitelist.tsv"), String::from("blacklist.tsv")];
    for suffix in suffixes {
        let path = root_path.with_extension(suffix);
        check_existance(&path.as_path());
    }
    ()
}

fn build_mutation_template(root: &String) -> String {
    eprintln!("{:?}", root);
//    1. read in all files:
//    template
    let mut html = String::from(include_str!("/home/ftabaro/IdeaProjects/RustyVCF/assets/mutations.html"));
//    js
    let d3 = String::from(include_str!("/home/ftabaro/IdeaProjects/RustyVCF/assets/d3.min.js"));
    let xlsx = String::from(include_str!("/home/ftabaro/IdeaProjects/RustyVCF/assets/xlsx.core.min.js"));
//    vcf
    let data_path = format!("/home/ftabaro/IdeaProjects/RustyVCF/data/{}.vcf.gz", &root);
    let vcf = String::from(include_str!(data_path));
//    blacklist
    let blacklist_path = format!("/home/ftabaro/IdeaProjects/RustyVCF/data/{}.blacklist.tsv", &root);
    let blacklist = String::from(include_str!(blacklist_path));
//    whitelist
    let whitelist_path = Path::from(format!("/home/ftabaro/IdeaProjects/RustyVCF/data/{}.whitelist.tsv", &root));
    let whitelist = match whitelist_path.to_str() {
        Some(p) => String::from(include_str!(p)),
        _ => panic!("Could not read whitelist file.")
    };

//    2. make all substitutions
    html = html
        .replace("{{xlsx_lib}}", xlsx)
        .replace("{{d3_lib}}", d3)
        .replace("{{vcf_data}}", vcf)
        .replace("{{blacklist_data}}", blacklist)
        .replace("{{whitelist_data}}", whitelist);
    html
}

fn main() {
    let addr: String = format!("{}:{}", URL, PORT);

    println!("Server listening at {}", &addr);

    start_server(addr, move |request| {
        router!(request,
                (GET)(/) => {
                    let d = get_datasets(DATA_PATH);
                    match d {
                        Ok(available_datasets) => Response::html(build_index_template(available_datasets)),
                        Err(_) => Response::text("").with_status_code(500),
                    }
                },

//            (GET)(/assets/{asset_type: String}/{asset_name: String}) => {
//                let mut folder = assets_path;
//                folder = match asset_type.as_ref() {
//                    "js" | "css" => format!("{}/{}", &folder, asset_type),
//                    _ => return Response::empty_404()
//                };
//                if let Some(request) = request.remove_prefix(format!("/assets/{}",&asset_type).as_ref()) {
//                    return match_assets(&request, &folder);
//                }
//                Response::empty_404()
//            },

                (GET)(/{ dataset: String } / { vcf: String }) => {
                    let root = format!("{}/{}",&dataset,&vcf);
                    // 1. check whitelist and blacklist files
                    check_auxiliary_files(&root);
                    // 2. generate template and return response
                    Response::html(build_mutation_template(&root))
                },

                (POST)(/update_mutation_blacklist) => {
                    Response::text("post to update_blacklist")
                },

                _ => Response::empty_404()
        )
    });
}

