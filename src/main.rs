#[macro_use]
extern crate rouille;

use rouille::{Response, start_server};

#[macro_use]
extern crate horrorshow;

use horrorshow::prelude::*;
use horrorshow::helper::doctype;

use std::fs::{read_dir, ReadDir};
use std::path::Path;

const PORT: &'static str = "8080";
const URL: &'static str = "localhost";
const DATA_PATH: &'static str = "/home/ftabaro/IdeaProjects/RustyVCF/data";
//const ASSETS_PATH: &'static str = "/home/ftabaro/IdeaProjects/RustyVCF/assets";

fn do_read_dir(p: &Path) -> ReadDir {
    read_dir(p).unwrap()
}

fn main() {
    let port: &str = &PORT[..];
    let url: &str = &URL[..];
    let data_path: &str = &DATA_PATH[..];
//    let assets_path: &str = &ASSETS_PATH[..];

    let addr: String = format!("{}:{}", url, port);

    println!("Server listening at {}", &addr);

    start_server(addr, move |request| {
        router!(request,
            (GET)(/) => {
                let mut html = include_str!("/home/ftabaro/IdeaProjects/RustyVCF/assets/html/index.tpl");
                let outer_list_template = "<li>{{dataset_name}}<ul>{{vcf_list}}</ul>";
                let inner_list_template = "<li><a href='/{{dataset_name}}/{{vcf_name}}'>{{vcf_name}}</a></li>";
                let mut outer_list = "";
                let paths = do_read_dir(Path::new(&data_path));
                for path in paths {
                    let unwrapped_path = path.unwrap();
                    let dataset_name = &unwrapped_path.file_name().into_string().unwrap();
                    let mut inner_list = "";
                    let mut outer_list_element = outer_list_template.clone();
                    let files = do_read_dir(&unwrapped_path.path());
                    for file in files {
                        let filename = file.unwrap().file_name().into_string().unwrap();
                        if filename.ends_with("vcf") {
                            let mut inner_list_element = inner_list_template.clone();
                            let inner_list_element = &inner_list_element.replace("{{vcf_name}}", &filename);
                            let inner_list_element = &inner_list_element.replace("{{dataset_name}}", &dataset_name);
                            let inner_list = &format!("{}{}", &inner_list, &inner_list_element);
                        }
                    }
                    let outer_list_element = &outer_list_element.replace("{{dataset_name}}", &dataset_name);
                    let outer_list_element = &outer_list_element.replace("{{vcf_list}}", &inner_list);
                    let outer_list = &format!("{}{}", &outer_list, &outer_list_element);
                }
                let html = html.replace(&"{{dataset_list}}", &outer_list);
                Response::text(html)
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


            (GET)(/{dataset: String}/{vcf: String}) => {
                Response::html(html!{
                    : doctype::HTML;
                    html {
                        head {
                            title : format!("{} > {}", &dataset, &vcf);
                            link(rel="stylesheet", type="text/css", href="../assets/css/viewer.css");
                        }
                        body {
                            div(id="options") {
                                label {
                                    input(id="show_silent", type="checkbox");
                                    : "Show silent mutations"
                                }
                                br;
                                label {
                                    input(id="show_blacklisted", type="checkbox");
                                    : "Show blacklisted mutations"
                                }
                                br;br;
                                button(id="export_xls", type="button") { : "Export XLSX" }
                            }
                            script(type="text/javascript", src="/assets/js/xlsx.core.min.js");
                            script(type="text/javascript", src="/assets/js/d3.min.js");
                            script(type="text/javascript", src="/assets/js/viewer.js");
                        }
                    }
                }.into_string().unwrap())
            },

            (POST)(/update_mutation_blacklist) => {
                Response::text("post to update_blacklist")
            },

            _ => Response::empty_404()
        )
    });
}

