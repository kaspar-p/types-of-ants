/// Options for creating a node in a deployment DAG.
///
/// Both fields are opaque strings to the engine — it stores them and gives them back.
/// The application layer serializes/deserializes `event` as JSON.
/// The engine only uses `mutates` for equality comparison (FIFO scheduling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeOptions {
    pub event: String,
    pub mutates: Option<String>,
}
