use crate::{db::Database, tests};
use reqwest;

pub fn get_all_tests(database: &Database) -> impl Fn() -> Result<(), reqwest::Error> {
    return tests::ping::ping_test(database);
}
