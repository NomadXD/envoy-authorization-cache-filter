#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate lazy_static;

use lazy_static::lazy_static;
use log::{debug, info, trace, warn};
use rocket::http::Status;
use rocket_contrib::json::Json;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
// #[get("/auth")]
// fn auth() -> &'static str {
//     "Hello, world!"
// }

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<&'static str, RateLimit>> = {
        let mut m = HashMap::new();
        m.insert("/foo", RateLimit::new(0, 100));
        m.insert("/bar", RateLimit::new(0, 50));
        m.insert("/baz", RateLimit::new(0, 200));
        Mutex::new(m)
    };
}

#[derive(Deserialize, Debug)]
struct Token {
    token: String,
    path: String,
}

#[post("/auth", format = "json", data = "<check_request>")]
fn authenticate(check_request: Json<Token>) -> Status {
    info!("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    let token: String = check_request.into_inner().token;
    let map = HASHMAP.lock().unwrap();
    let rate = map.get("/foo").unwrap();
    println!("Rate : {}", rate.clone().count);
    println!("Rate : {}", rate.clone().quota);
    if token == "AUTH_TOKEN" {
        return Status::Ok;
    } else {
        return Status::Unauthorized;
    };
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RateLimit {
    count: u32,
    quota: u32,
}

impl RateLimit {
    /// Creates a new RateLimit
    fn new(count: u32, quota: u32) -> RateLimit {
        RateLimit {
            count: count.clone(),
            quota: quota.clone(),
        }
    }
}

fn main() {
    let mut rate_limit: HashMap<&str, RateLimit> = HashMap::new();
    rate_limit.insert("/foo", RateLimit::new(0, 100));
    rate_limit.insert("/bar", RateLimit::new(0, 50));

    rocket::ignite()
        .manage(rate_limit)
        .mount("/", routes![authenticate])
        .launch();
}
