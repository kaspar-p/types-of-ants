use anthill::{build_artifacts, get_root};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let root = get_root()?;
    dbg!(&root);
    build_artifacts(&root).await;
    Ok(())
}
