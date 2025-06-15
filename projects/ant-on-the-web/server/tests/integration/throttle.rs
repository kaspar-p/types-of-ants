use crate::fixture::test_router_no_auth;
use futures::StreamExt;
use http::StatusCode;
use tracing::error;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn many_parallel_requests_get_429_too_many_requests() {
    let fixture = test_router_no_auth().await;

    const NUM_REQUESTS: usize = 100;

    let urls = vec!["/ping"; NUM_REQUESTS];
    let client = fixture.client;

    let responses = futures::stream::iter(urls)
        .map(|url| {
            let client = client.clone();
            tokio::spawn(async move { client.get(url).send().await })
        })
        .buffer_unordered(NUM_REQUESTS);

    // let r = responses
    let throttles = responses
        .filter_map(|res| async {
            let resp = res.unwrap();
            match resp.status() {
                StatusCode::TOO_MANY_REQUESTS => Some(resp.text().await),
                StatusCode::OK => None,
                _ => {
                    error!("{} {}", resp.status(), resp.text().await);
                    assert!(false);
                    None
                }
            }
        })
        .collect::<Vec<String>>()
        .await;

    assert!(throttles.len() >= 1);
    assert_eq!(throttles.first().unwrap(), "Throttling limit reached.");
}
