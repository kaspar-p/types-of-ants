use std::io::{self, Write};

use ant_on_the_web::ants::{AntId, AntReleaseRequest, UnreleasedAntsResponse};
use clap::Parser;
use rand::{rng, seq::SliceRandom};
use reqwest::StatusCode;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    endpoint: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let client = reqwest::Client::new();

    let endpoint = args
        .endpoint
        .unwrap_or("https://typesofants.org".to_string());

    let unreleased_ants: UnreleasedAntsResponse = {
        let req = client
            .get(format!("{endpoint}/api/ants/unreleased-ants?page=0"))
            .send()
            .await
            .expect("unreleased-ants");
        assert_eq!(req.status(), StatusCode::OK);

        req.json().await.unwrap()
    };

    let mut declined_ants: Vec<AntId> = vec![];
    let mut released_ants: Vec<AntReleaseRequest> = vec![];
    let mut skipped_ants: Vec<AntId> = vec![];

    let mut ants = unreleased_ants.ants;
    ants.shuffle(&mut rng());

    for (i, ant) in ants.iter().enumerate() {
        println!(
            "
[i:{i}, total:{}, yes:{}, no:{}, skipped:{}] [{}]
    [{}]",
            ants.len(),
            released_ants.len(),
            declined_ants.len(),
            skipped_ants.len(),
            ant.created_by_username,
            ant.ant_name
        );
        print!("(content/.y[es]/.n[o]/.s[kip]/.d[one]): ");
        io::stdout().flush().unwrap();

        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();

        match buffer.as_str() {
            b if b.starts_with(".n") || b == ".no" => declined_ants.push(ant.ant_id),
            b if b.starts_with(".y") || b == ".yes" => released_ants.push(AntReleaseRequest {
                ant_id: ant.ant_id,
                overwrite_content: None,
            }),
            b if b.starts_with(".s") || b == ".skip" => {
                skipped_ants.push(ant.ant_id);
                continue;
            }
            b if b.starts_with(".d") || b == ".done" => break,
            _ => released_ants.push(AntReleaseRequest {
                ant_id: ant.ant_id,
                overwrite_content: Some(buffer),
            }),
        }
    }

    println!("Hello, world!");
}
