use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
pub enum Service {
    #[strum(serialize = "ant-data-farm.service.ant")]
    AntDataFarm,

    #[strum(serialize = "ant-on-the-web.service.ant")]
    AntOnTheWeb,

    #[strum(serialize = "ant-looking-pretty.service.ant")]
    AntLookingPretty,

    #[strum(serialize = "ant-fs.service.ant")]
    AntFs,

    #[strum(serialize = "ant-backing-it-up.service.ant")]
    AntBackingItUp,

    #[strum(serialize = "ant-backing-it-up-db.service.ant")]
    AntBackingItUpDb,

    #[strum(serialize = "ant-gateway.service.ant")]
    AntGateway,

    #[strum(serialize = "ant-host-agent.service.ant")]
    AntHostAgent,

    #[strum(serialize = "ant-matchmaker.service.ant")]
    AntMatchmaker,

    #[strum(serialize = "ant-who-tweets.service.ant")]
    AntWhoTweets,

    #[strum(serialize = "ant-monitor.service.ant")]
    AntMonitor,

    #[strum(serialize = "ant-monitor-fe.service.ant")]
    AntMonitorFe,

    #[strum(serialize = "ant-naming-domains.service.ant")]
    AntNamingDomains,

    #[strum(serialize = "ant-worker-node-metrics-exporter.service.ant")]
    AntWorkerNodeMetricsExporter,

    #[strum(serialize = "ant-measuring-the-database.service.ant")]
    AntMeasuringTheDatabase,

    #[strum(serialize = "ant-siren.service.ant")]
    AntSiren,

    #[strum(serialize = "ant-sawmill.service.ant")]
    AntSawmill,

    #[strum(serialize = "ant-lumberjack.service.ant")]
    AntLumberjack,

    #[strum(serialize = "ant-just-checking-in.service.ant")]
    AntJustCheckingIn,
}
