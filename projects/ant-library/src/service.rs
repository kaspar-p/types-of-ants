use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
pub enum Service {
    #[strum(serialize = "ant-data-farm")]
    AntDataFarm,

    #[strum(serialize = "ant-on-the-web")]
    AntOnTheWeb,

    #[strum(serialize = "ant-looking-pretty")]
    AntLookingPretty,

    #[strum(serialize = "ant-fs")]
    AntFs,

    #[strum(serialize = "ant-backing-it-up")]
    AntBackingItUp,

    #[strum(serialize = "ant-backing-it-up-db")]
    AntBackingItUpDb,

    #[strum(serialize = "ant-gateway")]
    AntGateway,

    #[strum(serialize = "ant-host-agent")]
    AntHostAgent,

    #[strum(serialize = "ant-matchmaker")]
    AntMatchmaker,

    #[strum(serialize = "ant-who-tweets")]
    AntWhoTweets,

    #[strum(serialize = "ant-monitor")]
    AntMonitor,

    #[strum(serialize = "ant-monitor-fe")]
    AntMonitorFe,

    #[strum(serialize = "ant-naming-domains")]
    AntNamingDomains,

    #[strum(serialize = "ant-worker-node-metrics-exporter")]
    AntWorkerNodeMetricsExporter,

    #[strum(serialize = "ant-measuring-the-database")]
    AntMeasuringTheDatabase,

    #[strum(serialize = "ant-siren")]
    AntSiren,

    #[strum(serialize = "ant-sawmill")]
    AntSawmill,

    #[strum(serialize = "ant-lumberjack")]
    AntLumberjack,

    #[strum(serialize = "ant-just-checking-in")]
    AntJustCheckingIn,
}
