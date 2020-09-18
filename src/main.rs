mod graph;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    graph::graph_results().await?;
    Ok(())
}



