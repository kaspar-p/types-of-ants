use ant_host_agent::routes::service::InstallServiceRequest;
use hyper::StatusCode;
use stdext::function_name;
use tracing_test::traced_test;

use crate::fixture::TestFixture;

#[tokio::test]
#[traced_test]
async fn install_file() {
    let fixture = TestFixture::new(function_name!()).await;

    {
        let req = InstallServiceRequest {
            project: "proj1".to_string(),
            version: "v1".to_string(),
            is_docker: false,
        };

        let response = fixture
            .client
            .post("/service/service-installation")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
