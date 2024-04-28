use futures::future::BoxFuture;

use crate::tests::ping::StatusData;

type TestRunner = fn(bool) -> BoxFuture<'static, Vec<StatusData>>;

pub fn get_all_tests() -> Vec<TestRunner> {
    return vec![
        |enable: bool| Box::pin(crate::tests::ping::ping_test(enable)),
        |enable: bool| Box::pin(crate::tests::pinghost::pinghost_test(enable)),
    ];
}
