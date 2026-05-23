//! Example: Filtering remote/temporary printers.

use cpdb_rs::{CpdbClient, DiscoveryEvent};
use futures_util::StreamExt;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;
    println!("Connected to {} backend(s).", client.backend_count());

    let stream_client = client.clone();
    tokio::spawn(async move {
        let mut stream = stream_client.discovery_stream().await.unwrap();
        while let Some(event) = stream.next().await {
            match event {
                DiscoveryEvent::PrinterAdded(p) => {
                    println!("    [SIGNAL] + ADDED: {} ({})", p.name, p.backend);
                }
                DiscoveryEvent::PrinterRemoved { id, backend } => {
                    println!("    [SIGNAL] - REMOVED: {} ({})", id, backend);
                }
                DiscoveryEvent::PrinterStateChanged { id, state, .. } => {
                    println!("    [SIGNAL] ~ STATE: {} -> {}", id, state);
                }
            }
        }
    });

    // Wait for initial `doListing` signals to settle
    sleep(Duration::from_millis(500)).await;

    println!("\n--- Hiding remote printers ---");
    client.show_remote_printers(false).await;
    sleep(Duration::from_millis(1500)).await;

    println!("\n--- Restoring remote printers ---");
    client.show_remote_printers(true).await;
    sleep(Duration::from_millis(1500)).await;

    println!("\n--- Hiding temporary printers ---");
    client.show_temporary_printers(false).await;
    sleep(Duration::from_millis(1500)).await;

    println!("\n--- Restoring temporary printers ---");
    client.show_temporary_printers(true).await;
    sleep(Duration::from_millis(1500)).await;

    println!("\nDone filtering.");
    Ok(())
}
