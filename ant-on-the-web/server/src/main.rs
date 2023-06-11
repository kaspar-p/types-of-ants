#[macro_use]
extern crate rocket;

#[rocket::get("/")]
fn index() -> &'static str {
    return "Hello, world!\n";
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
