#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use log::{debug, info, trace, warn};
use rocket::http::Status;
use rocket_contrib::json::Json;
use serde::Deserialize;

#[get("/foo")]
fn foo() -> &'static str {
    "Hello from foo service!"
}

#[get("/bar")]
fn bar() -> &'static str {
    "Hello from bar service!"
}

#[get("/baz")]
fn baz() -> &'static str {
    "Hello from baz service!"
}

fn main() {
    rocket::ignite().mount("/", routes![foo, bar, baz]).launch();
}
