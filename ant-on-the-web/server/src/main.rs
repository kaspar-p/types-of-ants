#[macro_use]
extern crate rocket;
use std::fs::File;
use std::io::BufReader;

#[rocket::get("/")]
fn index() -> &'static str {
    return "Hello World!";
}

// #[rocket::get("/.well-known/acme-challenge/d7pvITjrjLvXFscDRJjvFJah00Gzas6SsvFLqn-UAQU")]
// fn certs() -> &'static str {
//     // let f: File = File::open("/etc/letsencrypt/live/beta.typesofants.key")?;
//     let mut buf_reader = BufReader::new(f);
//     let mut contents: String = String::new();
//     buf_reader.read_to_string(&mut contents)?;
//     return contents.as_str();
// }

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
