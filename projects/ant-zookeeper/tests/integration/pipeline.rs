use ant_library::host_architecture::HostArchitecture;
use ant_zookeeper::{
    err::AntZookeeperError,
    routes::pipeline::{
        AddHostToHostGroupRequest, CreateHostGroupRequest, CreateHostGroupResponse,
        GetHostGroupResponse, GetPipelineResponse, PutPipelineRequest, PutPipelineStage,
        RemoveHostFromHostGroupRequest,
    },
};
use http::StatusCode;
use stdext::function_name;
use tokio::test;
use tracing_test::traced_test;

use crate::fixture::{self, Fixture};

#[test]
#[traced_test]
async fn pipeline_host_group_host_group_get_returns_4xx() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    {
        let res = fixture
            .client
            .get("/pipeline/host-group/host-group?name=some-bad-name")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST)
    }
}

#[test]
#[traced_test]
async fn pipeline_host_group_host_post_returns_4xx_if_no_host_group() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    {
        let req = AddHostToHostGroupRequest {
            host_group_id: "bad-id".to_string(),
            host_id: "bad-id".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST)
    }

    {
        let host_group_id = {
            let req = CreateHostGroupRequest {
                name: "group1".to_string(),
                environment: "beta".to_string(),
            };

            let res = fixture
                .client
                .post("/pipeline/host-group/host-group")
                .json(&req)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);

            let body: CreateHostGroupResponse = res.json().await;

            body.id
        };

        let req = AddHostToHostGroupRequest {
            host_group_id,
            host_id: "bad-id".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST)
    }
}

#[test]
#[traced_test]
async fn pipeline_host_group_host_delete_returns_4xx_if_no_host_group() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    {
        let req = RemoveHostFromHostGroupRequest {
            host_group_id: "bad-id".to_string(),
            host_id: "bad-id".to_string(),
        };

        let res = fixture
            .client
            .delete("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST)
    }

    {
        let host_group_id = {
            let req = CreateHostGroupRequest {
                name: "group1".to_string(),
                environment: "beta".to_string(),
            };

            let res = fixture
                .client
                .post("/pipeline/host-group/host-group")
                .json(&req)
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);

            let body: CreateHostGroupResponse = res.json().await;

            body.id
        };

        let req = RemoveHostFromHostGroupRequest {
            host_group_id,
            host_id: "bad-id".to_string(),
        };

        let res = fixture
            .client
            .delete("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST)
    }
}

#[test]
#[traced_test]
async fn pipeline_host_group_host_post_returns_200_if_double_add() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    // create group
    let host_group_id = {
        let req = CreateHostGroupRequest {
            name: "group1".to_string(),
            environment: "beta".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host-group")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: CreateHostGroupResponse = res.json().await;

        body.id
    };

    // add host
    {
        let req = AddHostToHostGroupRequest {
            host_group_id: host_group_id.clone(),
            host_id: "antworker000.hosts.typesofants.org".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // double add works fine
    {
        let req = AddHostToHostGroupRequest {
            host_group_id: host_group_id.clone(),
            host_id: "antworker000.hosts.typesofants.org".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[test]
#[traced_test]
async fn pipeline_host_group_host_post_then_delete_returns_200() {
    let fixture = fixture::Fixture::new(function_name!()).await;

    let host_group_id = {
        let req = CreateHostGroupRequest {
            name: "group1".to_string(),
            environment: "beta".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host-group")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: CreateHostGroupResponse = res.json().await;

        body.id
    };

    // add
    {
        let req = AddHostToHostGroupRequest {
            host_group_id: host_group_id.clone(),
            host_id: "antworker000.hosts.typesofants.org".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // read 1
    {
        let res = fixture
            .client
            .get("/pipeline/host-group/host-group?name=group1")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: GetHostGroupResponse = res.json().await;

        assert_eq!(body.host_group.name, "group1");
        assert!(body.host_group.description.is_none());
        assert_eq!(body.host_group.id, host_group_id);
        assert_eq!(
            body.host_group.hosts.first().unwrap().name,
            "antworker000.hosts.typesofants.org"
        );
        assert_eq!(body.host_group.hosts.len(), 1);
    }

    // delete
    {
        let req = RemoveHostFromHostGroupRequest {
            host_group_id: host_group_id.clone(),
            host_id: "antworker000.hosts.typesofants.org".to_string(),
        };

        let res = fixture
            .client
            .delete("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // read 2
    {
        let res = fixture
            .client
            .get("/pipeline/host-group/host-group?name=group1")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: GetHostGroupResponse = res.json().await;

        assert_eq!(body.host_group.name, "group1");
        assert!(body.host_group.description.is_none());
        assert_eq!(body.host_group.id, host_group_id);
        assert_eq!(body.host_group.hosts.len(), 0);
    }
}

#[test]
#[traced_test]
async fn pipeline_pipeline_post_returns_4xx_bad_host_group() {
    let fixture = Fixture::new(function_name!()).await;

    let req = PutPipelineRequest {
        project: "ant-data-farm".to_string(),
        stages: vec![PutPipelineStage {
            name: "beta".to_string(),
            host_group_id: "bad-id".to_string(),
        }],
    };

    let res = fixture
        .client
        .post("/pipeline/pipeline")
        .json(&req)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    assert!(matches!(
        res.json::<AntZookeeperError>().await,
        AntZookeeperError::ValidationError(msg) if msg == "No such host group: bad-id",
    ));
}

#[test]
#[traced_test]
async fn pipeline_pipeline_post_returns_200_empty_pipeline() {
    let fixture = Fixture::new(function_name!()).await;

    let req = PutPipelineRequest {
        project: "ant-data-farm".to_string(),
        stages: vec![],
    };

    let res = fixture
        .client
        .post("/pipeline/pipeline")
        .json(&req)
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
#[traced_test]
async fn pipeline_pipeline_post_returns_4xx_for_empty_group() {
    let fixture = Fixture::new(function_name!()).await;

    // make group
    let host_group_id = {
        let req = CreateHostGroupRequest {
            name: "ant-data-farm/beta".to_string(),
            environment: "beta".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host-group")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: CreateHostGroupResponse = res.json().await;

        body.id
    };

    {
        let req = PutPipelineRequest {
            project: "ant-data-farm".to_string(),
            stages: vec![PutPipelineStage {
                name: "stage1".to_string(),
                host_group_id: host_group_id.clone(),
            }],
        };

        let res = fixture
            .client
            .post("/pipeline/pipeline")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert!(
            matches!(res.json::<AntZookeeperError>().await, 
            AntZookeeperError::ValidationError(msg) if msg ==
            format!("Host group {host_group_id} cannot be added to a pipeline because it has no hosts."))
        )
        // assert!(err_msg.contains("Host group "));
        // assert!(err_msg.contains(&host_group_id));
        // assert!(err_msg.contains(" cannot be added to a pipeline because it has no hosts."));
    }
}

#[test]
#[traced_test]
async fn pipeline_pipeline_post_returns_200_full_pipeline() {
    let fixture = Fixture::new(function_name!()).await;

    // make group
    let host_group_id = {
        let req = CreateHostGroupRequest {
            name: "ant-data-farm/beta".to_string(),
            environment: "beta".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host-group")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: CreateHostGroupResponse = res.json().await;

        body.id
    };

    // Add to group
    {
        let req = AddHostToHostGroupRequest {
            host_group_id: host_group_id.clone(),
            host_id: "antworker000.hosts.typesofants.org".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // put pipeline
    {
        let req = PutPipelineRequest {
            project: "ant-data-farm".to_string(),
            stages: vec![PutPipelineStage {
                name: "stage1".to_string(),
                host_group_id: host_group_id.clone(),
            }],
        };

        let res = fixture
            .client
            .post("/pipeline/pipeline")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // get pipeline
    {
        let res = fixture
            .client
            .get("/pipeline/pipeline?project=ant-data-farm")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: GetPipelineResponse = res.json().await;

        assert_eq!(body.project, "ant-data-farm");
        assert_eq!(body.stages[0].stage_name, "build");

        assert_eq!(body.stages[1].stage_name, "stage1");
        let deploy_stage = body.stages[1].clone().deploy_stage();
        assert_eq!(
            deploy_stage.host_group.hosts[0].name,
            "antworker000.hosts.typesofants.org"
        );
        assert_eq!(
            deploy_stage.host_group.hosts[0].arch,
            HostArchitecture::ArmV7
        );
        assert_eq!(deploy_stage.host_group.hosts.len(), 1);
        assert_eq!(body.stages.len(), 2);
    }
}

#[test]
#[traced_test]
async fn pipeline_pipeline_post_returns_200_for_different_projects() {
    let fixture = Fixture::new(function_name!()).await;

    // make group ant-data-farm/beta
    let ant_data_farm_group = {
        let req = CreateHostGroupRequest {
            name: "ant-data-farm/beta".to_string(),
            environment: "beta".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host-group")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: CreateHostGroupResponse = res.json().await;

        body.id
    };

    // Add 000 to ant-data-farm/beta
    {
        let req = AddHostToHostGroupRequest {
            host_group_id: ant_data_farm_group.clone(),
            host_id: "antworker000.hosts.typesofants.org".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // make group ant-on-the-web/beta
    let ant_on_the_web_group = {
        let req = CreateHostGroupRequest {
            name: "ant-on-the-web/beta".to_string(),
            environment: "beta".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host-group")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
        let body: CreateHostGroupResponse = res.json().await;

        body.id
    };

    // Add 000 to ant-on-the-web/beta
    {
        let req = AddHostToHostGroupRequest {
            host_group_id: ant_on_the_web_group.clone(),
            host_id: "antworker000.hosts.typesofants.org".to_string(),
        };

        let res = fixture
            .client
            .post("/pipeline/host-group/host")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // make pipeline for ant-data-farm
    {
        let req = PutPipelineRequest {
            project: "ant-data-farm".to_string(),
            stages: vec![PutPipelineStage {
                name: "beta-stage".to_string(),
                host_group_id: ant_data_farm_group,
            }],
        };

        let res = fixture
            .client
            .post("/pipeline/pipeline")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // get pipeline for ant-data-farm
    {
        let res = fixture
            .client
            .get("/pipeline/pipeline?project=ant-data-farm")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: GetPipelineResponse = res.json().await;

        assert_eq!(body.project, "ant-data-farm");
        assert_eq!(body.stages[0].stage_name, "build");

        assert_eq!(body.stages[1].stage_name, "beta-stage");
        let deploy_stage = body.stages[1].clone().deploy_stage();
        assert_eq!(
            deploy_stage.host_group.hosts[0].name,
            "antworker000.hosts.typesofants.org"
        );
        assert_eq!(
            deploy_stage.host_group.hosts[0].arch,
            HostArchitecture::ArmV7
        );
        assert_eq!(deploy_stage.host_group.hosts.len(), 1);
        assert_eq!(body.stages.len(), 2);
    }

    // make pipeline for ant-on-the-web
    {
        let req = PutPipelineRequest {
            project: "ant-on-the-web".to_string(),
            stages: vec![PutPipelineStage {
                name: "beta-website".to_string(),
                host_group_id: ant_on_the_web_group,
            }],
        };

        let res = fixture
            .client
            .post("/pipeline/pipeline")
            .json(&req)
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);
    }

    // get pipeline for ant-on-the-web
    {
        let res = fixture
            .client
            .get("/pipeline/pipeline?project=ant-on-the-web")
            .send()
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let body: GetPipelineResponse = res.json().await;

        assert_eq!(body.project, "ant-on-the-web");
        assert_eq!(body.stages[0].stage_name, "build");

        assert_eq!(body.stages[1].stage_name, "beta-website");
        let deploy_stage = body.stages[1].clone().deploy_stage();
        assert_eq!(
            deploy_stage.host_group.hosts[0].name,
            "antworker000.hosts.typesofants.org"
        );
        assert_eq!(
            deploy_stage.host_group.hosts[0].arch,
            HostArchitecture::ArmV7
        );
        assert_eq!(deploy_stage.host_group.hosts.len(), 1);
        assert_eq!(body.stages.len(), 2);
    }
}
