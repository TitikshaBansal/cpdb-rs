use cpdb_rs::CpdbClient;
use cpdb_rs::events::DiscoveryEvent;
use futures_util::StreamExt;

/// This example shows how to use the CpdbClient to discover printers and print their details.
/// It also shows how to listen for live discovery events.
#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;
    println!("Connected to {} backend(s).\n", client.backend_count());

    // Initial population via GetAllPrinters
    let printers = client.get_all_printers().await?;
    println!("=== All Printers ({} found) ===", printers.len());
    for p in &printers {
        println!(
            "  {} [{}] - {} (state={}, accepting={})",
            p.name, p.id, p.make_model, p.state, p.accepting_jobs
        );
    }

    // Default printer
    match client.get_default_printer("CUPS").await {
        Ok(default) => {
            let display = if default.is_empty() || default == "[Invalid UTF-8]" || default == "NA" {
                "Not Set"
            } else {
                &default
            };
            println!("\nDefault CUPS printer: {display}");
        }
        Err(_) => println!("\nDefault CUPS printer: Not Set"),
    }

    // Fetch details for the first accepting printer
    if let Some(p) = printers.iter().find(|p| p.is_ready()) {
        println!("\n=== Details for '{}' ===", p.name);
        match client.get_printer_details(&p.id, &p.backend).await {
            Ok((options, media)) => {
                println!("Options ({} total):", options.len());
                for opt in options.iter().take(8) {
                    println!(
                        "  {}: default='{}', choices=[{}]",
                        opt.name,
                        opt.default_value,
                        opt.supported_values.join(", ")
                    );
                }
                println!("Media ({} total):", media.len());
                for m in media.iter().take(5) {
                    println!(
                        "  {}: {}×{} mm, {} margin(s)",
                        m.name,
                        m.width as f64 / 100.0,
                        m.length as f64 / 100.0,
                        m.margins.len()
                    );
                }
            }
            Err(e) => eprintln!("  Failed to get details: {e}"),
        }
    } else {
        println!("\nNo ready printers to query details for.");
    }

    // Live discovery stream
    println!("\n=== Listening for live discovery events ===");
    println!(
        "(Try: sudo lpadmin -p TestPrinter -E -v ipp://localhost/printers/test -m everywhere)"
    );
    println!("(Press Ctrl+C to stop)\n");

    // Spawn a background task to keep backends alive
    let keep_alive_client = client.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            keep_alive_client.keep_alive_all().await;
        }
    });

    let mut stream = client.discovery_stream().await?;
    while let Some(event) = stream.next().await {
        match &event {
            DiscoveryEvent::PrinterAdded(snap) => {
                println!(
                    "+ ADDED: {} [{}] backend={}",
                    snap.name, snap.id, snap.backend
                );
            }
            DiscoveryEvent::PrinterRemoved { id, backend } => {
                println!("- REMOVED: id={id}, backend={backend}");
            }
            DiscoveryEvent::PrinterStateChanged { id, state, .. } => {
                println!("~ STATE: id={id}, state={state}");
            }
        }
    }

    Ok(())
}
