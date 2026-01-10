use std::fs;

use ant_host_agent::routes::secret::{
    DeleteSecretRequest, PeekSecretRequest, PeekSecretResponse, PutSecretRequest,
};
use hyper::StatusCode;
use serde_json::json;
use stdext::function_name;
use tracing_test::traced_test;

use crate::fixture::TestFixture;

#[tokio::test]
#[traced_test]
async fn bytes_smoke() {
    let fixture = TestFixture::new(function_name!(), None).await;

    {
        let json = json!({
            "name": "name",
            // base64("secret string value")
            "value": "c2VjcmV0IHN0cmluZyB2YWx1ZQ=="
        });

        let response = fixture
            .client
            .post("/secret/secret")
            .header("Content-Type", "application/json")
            .body(json.to_string())
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(
            fs::read_to_string(fixture.test_root_dir.join("secrets").join("name.secret")).unwrap(),
            "secret string value"
        );
    }
}

#[tokio::test]
#[traced_test]
async fn put_secret_then_peek_works() {
    let fixture = TestFixture::new(function_name!(), None).await;

    {
        let req = PutSecretRequest {
            name: "name".to_string(),
            value: "new secret value".as_bytes().to_vec(),
        };

        let response = fixture
            .client
            .post("/secret/secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    {
        let req = PeekSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture.client.get("/secret/secret").json(&req).send().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeekSecretResponse = response.json().await;
        assert_eq!(body.secret_exists, true);
    }
}

#[tokio::test]
#[traced_test]
async fn delete_secret_works_if_no_secret() {
    let fixture = TestFixture::new(function_name!(), None).await;

    {
        let req = PeekSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture.client.get("/secret/secret").json(&req).send().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeekSecretResponse = response.json().await;
        assert_eq!(body.secret_exists, false);
    }

    {
        let req = DeleteSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture
            .client
            .delete("/secret/secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    {
        let req = PeekSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture.client.get("/secret/secret").json(&req).send().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeekSecretResponse = response.json().await;
        assert_eq!(body.secret_exists, false);
    }
}

#[tokio::test]
#[traced_test]
async fn put_then_delete_works() {
    let fixture = TestFixture::new(function_name!(), None).await;

    {
        let req = PeekSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture.client.get("/secret/secret").json(&req).send().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeekSecretResponse = response.json().await;
        assert_eq!(body.secret_exists, false);
    }

    {
        let req = PutSecretRequest {
            name: "name".to_string(),
            value: "new secret value".as_bytes().to_vec(),
        };

        let response = fixture
            .client
            .post("/secret/secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    {
        let req = PeekSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture.client.get("/secret/secret").json(&req).send().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeekSecretResponse = response.json().await;
        assert_eq!(body.secret_exists, true);
    }

    {
        let req = DeleteSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture
            .client
            .delete("/secret/secret")
            .json(&req)
            .send()
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    {
        let req = PeekSecretRequest {
            secret_name: "name".to_string(),
        };

        let response = fixture.client.get("/secret/secret").json(&req).send().await;

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeekSecretResponse = response.json().await;
        assert_eq!(body.secret_exists, false);
    }
}
