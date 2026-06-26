#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeOptions {
    pub is_unwind_boundary: bool,
    pub unwind_on_failure: bool,
}

impl Default for NodeOptions {
    fn default() -> Self {
        Self {
            is_unwind_boundary: false,
            unwind_on_failure: true,
        }
    }
}

/// Everything needed to create a node in a deployment DAG.
///
/// `event` and `mutates` are opaque strings to the engine — it stores them and gives them back.
/// The application layer serializes/deserializes `event` as JSON.
/// The engine only uses `mutates` for equality comparison (FIFO scheduling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSpec {
    pub event: String,
    pub mutates: Option<String>,
    pub options: NodeOptions,
}
