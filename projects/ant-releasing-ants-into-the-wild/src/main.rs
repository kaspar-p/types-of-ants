use std::{
    fmt::Debug,
    fs::File,
    io::{Read, Write, stdin},
    path::PathBuf,
};

use ant_on_the_web::ants::{
    Ant, AntId, AntReleaseRequest, CreateReleaseRequest, CreateReleaseResponse, DeclineAntRequest,
    DeclineAntResponse, UnreleasedAntsResponse,
};
use chrono::{Datelike, Local};
use clap::Parser;
use rand::{rng, seq::SliceRandom};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    endpoint: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
enum UserChoice {
    Release {
        ant: Ant,
        release: AntReleaseRequest,
    },
    Decline(Ant),
    Skip(Ant),
}

impl UserChoice {
    pub fn is_ant(&self, ant_id: &AntId) -> bool {
        match &self {
            UserChoice::Release { ant, .. } => ant.ant_id == *ant_id,
            UserChoice::Decline(ant) => ant.ant_id == *ant_id,
            UserChoice::Skip(ant) => ant.ant_id == *ant_id,
        }
    }

    pub fn is_released(&self) -> bool {
        match self {
            UserChoice::Release { .. } => true,
            _ => false,
        }
    }

    pub fn is_declined(&self) -> bool {
        match self {
            UserChoice::Decline(_) => true,
            _ => false,
        }
    }

    pub fn is_skipped(&self) -> bool {
        match self {
            UserChoice::Skip(_) => true,
            _ => false,
        }
    }
}

struct History {
    path: PathBuf,
    history: Vec<UserChoice>,
}

impl History {
    pub fn new(path: PathBuf) -> Self {
        let history = History::load(&path);
        History {
            path,
            history: history,
        }
    }

    fn load(path: &PathBuf) -> Vec<UserChoice> {
        let mut buf = String::new();
        match File::open(&path).map(|mut f| f.read_to_string(&mut buf).unwrap()) {
            Err(_) => vec![],
            Ok(_) => serde_json::de::from_str(&buf).unwrap(),
        }
    }

    pub fn num_released(&self) -> i32 {
        self.history.iter().filter(|c| c.is_released()).count() as i32
    }

    pub fn num_skipped(&self) -> i32 {
        self.history.iter().filter(|c| c.is_skipped()).count() as i32
    }

    pub fn num_declined(&self) -> i32 {
        self.history.iter().filter(|c| c.is_declined()).count() as i32
    }

    fn save(&self) {
        let buf = serde_json::ser::to_string(&self.history).unwrap();
        let mut file = File::create(&self.path).unwrap();
        file.write_all(buf.as_bytes()).unwrap();
        file.flush().unwrap();
    }

    pub fn push(&mut self, choice: UserChoice) {
        self.history.push(choice);
        self.save();
    }

    pub fn is_present(&self, ant: &AntId) -> bool {
        self.history.iter().find(|c| c.is_ant(&ant)).is_some()
    }

    pub fn pop(&mut self) -> Option<UserChoice> {
        let ret = self.history.pop();
        self.save();
        ret
    }

    pub fn dump(self) -> Vec<UserChoice> {
        self.history
    }
}

fn decide_ants_loop(unreleased_ants: Vec<Ant>, history: &mut History) {
    let mut ants = unreleased_ants;
    ants.shuffle(&mut rng());

    let ants: Vec<(usize, Ant)> = ants
        .into_iter()
        .filter(|a| !history.is_present(&a.ant_id))
        .enumerate()
        .collect();

    for (i, ant) in &ants {
        let prompt = format!(
            "
[i:{}, total:{}, yes:{}, no:{}, skipped:{}] [{}]
    [{}]
(content/.b[ack]/.y[es]/.n[o]/.s[kip]/.d[one]): ",
            i,
            ants.len(),
            history.num_released(),
            history.num_declined(),
            history.num_skipped(),
            ant.created_by_username,
            ant.ant_name,
        );

        let mut buffer = String::new();
        print!("{}", prompt);
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut buffer).unwrap();

        match buffer.as_str() {
            b if b.starts_with(".n") || b == ".no" => {
                println!("  declined!");
                history.push(UserChoice::Decline(ant.clone()));
            }
            b if b.starts_with(".y") || b == ".yes" => {
                println!("  accepted!");
                history.push(UserChoice::Release {
                    ant: ant.clone(),
                    release: AntReleaseRequest {
                        ant_id: ant.ant_id,
                        overwrite_content: None,
                    },
                });
            }
            b if b.starts_with(".s") || b == ".skip" => {
                println!("  skipped!");
                history.push(UserChoice::Skip(ant.clone()));
                continue;
            }
            b if b.starts_with(".d") || b == ".done" => {
                println!("  exiting!");
                break;
            }
            b if b.starts_with(".b") || b == ".back" => {
                let elem = history.pop().expect("cannot go back");
                println!("Removing: {elem:#?}");
            }
            _ => {
                println!("  replacing with: {}", buffer);
                history.push(UserChoice::Release {
                    ant: ant.clone(),
                    release: AntReleaseRequest {
                        ant_id: ant.ant_id,
                        overwrite_content: Some(buffer),
                    },
                });
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    let api_token =
        ant_library::secret::load_secret("typesofants_kaspar_api_token").expect("load api token");
    let cookie = format!("typesofants_auth=kaspar:{api_token}");

    let now = Local::now();
    let date_string = format!("ant-release.{}-{}-{}", now.year(), now.month(), now.day());

    let endpoint = args
        .endpoint
        .unwrap_or("https://www.typesofants.org".to_string());

    let file_name = format!(
        "{}.{}.json",
        endpoint.replace("/", "").replace(":", ""),
        date_string
    );

    let unreleased_ants: UnreleasedAntsResponse = {
        let req = reqwest::blocking::Client::new()
            .get(format!("{endpoint}/api/ants/unreleased-ants?page=0"))
            .header("Cookie", &cookie)
            .send()
            .expect("unreleased-ants");
        assert_eq!(req.status(), StatusCode::OK);

        req.json().unwrap()
    };

    let mut history = History::new(PathBuf::from(format!("./releases/{file_name}")));

    decide_ants_loop(unreleased_ants.ants, &mut history);

    println!("Thanks for choosing! To continue to send data to server, press ENTER:");
    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap();

    let mut future_release: Vec<AntReleaseRequest> = vec![];
    for choice in history.dump() {
        match choice {
            UserChoice::Skip(_) => continue,
            UserChoice::Decline(ant) => {
                println!("> Declining ant: {}", ant.ant_name);
                let req = reqwest::blocking::Client::new()
                    .post(format!("{endpoint}/api/ants/decline"))
                    .header("Cookie", &cookie)
                    .json(&DeclineAntRequest { ant_id: ant.ant_id });

                println!("{:?}", req);

                let res = req.send().expect("declining ant");

                println!("{:?}", res);

                if res.status() == StatusCode::BAD_REQUEST {
                    println!("skipping: {}", ant.ant_name);
                    continue;
                }
                assert_eq!(res.status(), StatusCode::OK);
                let body: DeclineAntResponse = res.json().expect("deserialize");

                println!("> Declined ant: {}, {}", ant.ant_name, body.declined_at);
            }
            UserChoice::Release { release, ant } => {
                println!("> Releasing ant: {}", ant.ant_name);
                future_release.push(release);
            }
        }
    }

    let req = CreateReleaseRequest {
        label: date_string,
        ants: future_release,
    };

    println!("{req:#?}");

    let res = reqwest::blocking::Client::new()
        .post(format!("{endpoint}/api/ants/release"))
        .json(&req)
        .header("Cookie", format!("typesofants_auth=kaspar:{api_token}"))
        .send()
        .expect("declining ant");
    let body: CreateReleaseResponse = res.json().expect("deserialize");

    println!("Congratulations, release created: {body:#?}");
}
