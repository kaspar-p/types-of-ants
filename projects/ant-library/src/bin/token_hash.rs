use std::env::args;

fn main() {
    let mut args = args();
    args.next();
    let arg = args.next().unwrap();

    println!("{}", ant_library::crypto::make_token_hash(&arg));
}
