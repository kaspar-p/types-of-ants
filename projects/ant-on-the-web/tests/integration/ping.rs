use crate::fixture::{FixtureOptions, TestFixture};
use http::StatusCode;

#[tokio::test]
async fn ping_works() {
    let fixture = TestFixture::new(FixtureOptions::new()).await;

    let res = fixture.client.get("/ping").send().await;

    assert_eq!(res.status(), StatusCode::OK);

    let x_ant = res
        .headers()
        .get("x-ant")
        .expect("x-ant header missing from /ping response");
    let x_ant_str = x_ant.to_str().expect("x-ant header is not valid UTF-8");
    assert!(!x_ant_str.is_empty(), "x-ant header must not be empty");
    assert!(
        x_ant_str.chars().all(|c| ('\x20'..='\x7e').contains(&c)),
        "x-ant header contains non-printable-ASCII characters: {x_ant_str:?}"
    );

    assert_eq!(res.text().await, "healthy ant");
}
