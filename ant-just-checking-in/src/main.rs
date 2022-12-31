mod emit;

use cronjob::CronJob;
use emit::emit_data;
use reqwest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the `CronJob` object.
    let mut cron: CronJob = CronJob::new("Test Cron", move |_name: &str| -> () {
        let urls: Vec<&str> = vec![
            "http://typesofants.org",
            "http://www.typesofants.org",
            "http://6krill.com",
        ];
        let client = reqwest::blocking::Client::new();

        on_cron(urls, &client);
    });

    // Run every 10 seconds
    cron.seconds("*/10");

    // Start the cronjob.
    cron.start_job();

    return Ok(());
}

// Our cronjob handler.
fn on_cron(urls: Vec<&str>, client: &reqwest::blocking::Client) -> () {
    for url in urls {
        let res = client.get(url).send();
        emit_data(url, res.unwrap());
    }
}
