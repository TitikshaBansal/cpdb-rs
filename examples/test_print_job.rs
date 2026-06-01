//! Example: Submit a print job via print_fd() with fallback to print_socket().
//!
//! Targets `dummy_printer` by default. Change PRINTER_ID for a different target.

use cpdb_rs::{CpdbClient, CpdbError};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

/// Change this to target a different printer.
const PRINTER_ID: &str = "dummy_printer";
const BACKEND: &str = "CUPS";

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;

    let printers = client.get_all_printers().await?;
    let target = printers.iter().find(|p| p.id == PRINTER_ID);
    match target {
        Some(p) => println!(
            "Target: {} [state={}, accepting={}]",
            p.name, p.state, p.accepting_jobs
        ),
        None => {
            eprintln!("Printer '{}' not found. Available:", PRINTER_ID);
            for p in &printers {
                eprintln!("  {} [{}]", p.name, p.id);
            }
            return Ok(());
        }
    }

    let settings = [("copies", "1"), ("media", "iso_a4_210x297mm")];
    let title = "cpdb-rs test page";

    println!("\nSubmitting job to '{}'...", PRINTER_ID);

    let postscript = b"%!PS-Adobe-3.0
%%Title: cpdb-rs test page
%%Pages: 1
%%EndComments

%%Page: 1 1
/Helvetica findfont 24 scalefont setfont
72 700 moveto
(Hello from cpdb-rs!) show
showpage
%%EOF
";

    match client.print_fd(PRINTER_ID, BACKEND, &settings, title).await {
        Ok((job_id, fd)) => {
            println!("printFd SUCCESS: Job ID: {}", job_id);

            // Convert the zbus OwnedFd into a std OwnedFd, then into a File to prevent double-drop
            let std_fd: std::os::fd::OwnedFd = fd.into();
            let mut file = std::fs::File::from(std_fd);
            use std::io::Write;
            file.write_all(postscript).expect("Failed to write to FD");
            drop(file); // Signals EOF

            println!("Document written to FD and stream closed.");
        }
        Err(CpdbError::DbusError(zbus::Error::MethodError(name, _, _)))
            if name.as_str() == "org.freedesktop.DBus.Error.UnknownMethod" =>
        {
            println!("printFd not supported by backend. Falling back to printSocket...");

            let (job_id, socket_path) = client
                .print_socket(PRINTER_ID, BACKEND, &settings, title)
                .await?;

            println!(
                "printSocket SUCCESS: Job ID: {}, Socket: {}",
                job_id, socket_path
            );

            let mut stream = UnixStream::connect(&socket_path)
                .await
                .expect("Failed to connect to print socket");

            stream
                .write_all(postscript)
                .await
                .expect("Failed to write to socket");

            stream.shutdown().await.unwrap(); // Signals EOF
            println!("Document written to socket and stream closed.");
        }
        Err(e) => {
            return Err(e);
        }
    }

    println!("\nCheck the job with: lpstat -o");

    Ok(())
}
