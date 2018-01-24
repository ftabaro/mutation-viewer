#[macro_use]
extern crate rouille;

use rouille::{Request, Response, start_server};
use std::net::{ToSocketAddrs, SocketAddr};

const URL: String = "localhost".into(); // same as URL: String = String::from("localhost");
const PORT: String = "8080".into();

fn main () {

    let addr = format!("{}:{}", URL, PORT);

    println!("Server listening at {}", &addr);

    start_server(addr, move |request: Request| {

        router!(request,
            (GET)(/) => {
                println!("{:?}", &request);
                {
                    rouille::match_assets(&request, "../assets");
                }
                Response::empty_404()
            },

            (GET)(/{dataset}/{vcf}/mutations) => {
                Response::text("this is the view page")
            },

            (POST)(/update_mutation_blacklist) => {
                Response::text("post to update_blacklist")
            },

            _ => Response::empty_404()
        )

    });

}

