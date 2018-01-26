#[macro_use]
extern crate rouille;

#[macro_use]
extern crate horrorshow;

mod index;

use rouille::{Request, Response, start_server};
//use std::fs::{File, read_dir};

const PORT: &'static str = "8080";
const URL: &'static str = "localhost";

// fn buildString(s: &str) -> String { &s[..] }

fn main () {

    let port: &str = &PORT[..];
    let url: &str = &URL[..];

    let addr = format!("{}:{}", url, port);

    println!("Server listening at {}", &addr);

    start_server(addr, move |request| {

        router!(request,
            (GET)(/) => {

                // let paths = read_dir("../data").unwrap();

                // println!("{:?}",paths);
                // for path in paths {
                //    println!("{:?}", path.unwrap().path())
                // }

                //let file = File::open("../assets/index.html").unwrap();

                Response::html(index::index())
            },

            (GET)(/{dataset: String}/{vcf: String}/mutations) => {
                println!("{}/{}",&dataset, &vcf);
                Response::text("this is the view page")
            },

            (POST)(/update_mutation_blacklist) => {
                Response::text("post to update_blacklist")
            },

            _ => Response::empty_404()
        )
    });
}

