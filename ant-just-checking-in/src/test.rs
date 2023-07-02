use std::future::Future;

use crate::tests::ping::StatusData;

pub fn get_all_tests() -> Vec<impl Future<Output = Vec<StatusData>>> {
    vec![crate::tests::ping::ping_test()]
}
