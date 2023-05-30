use std::future::Future;

use crate::tests::ping::StatusData;

pub fn get_all_tests() -> Vec<impl Future<Output = Vec<StatusData>>> {
    return vec![crate::tests::ping::ping_test()];
}
