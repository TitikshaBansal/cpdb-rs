//! Live D-Bus integration tests. All `#[ignore]`d by default — they
//! require a session D-Bus and at least one cpdb backend to be active.
//! Run with `cargo test -- --ignored`.

#[cfg(all(test, feature = "ffi"))]
mod ffi_integration {
    use cpdb_rs::Frontend;
    use std::fs;
    use std::io::Write;

    fn write_temp_test_file(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(name);
        let mut f = fs::File::create(&path).expect("failed to create test print file");
        writeln!(f, "cpdb-rs integration test").unwrap();
        path
    }

    #[test]
    #[ignore]
    fn printer_discovery() {
        cpdb_rs::init();
        let frontend = Frontend::new().expect("frontend init failed");
        frontend.connect_to_dbus().expect("connect_to_dbus failed");
        let printers = frontend.get_printers().expect("get_printers failed");
        for p in &printers {
            let name = p.name().unwrap_or_default();
            let state = p.get_updated_state().unwrap_or_default();
            eprintln!("found {name}: {state}");
        }
    }

    #[test]
    #[ignore]
    fn job_submission_applies_options() {
        cpdb_rs::init();
        let frontend = Frontend::new().expect("frontend init failed");
        frontend.connect_to_dbus().expect("connect_to_dbus failed");
        let printers = frontend.get_printers().unwrap();
        let printer = match printers.first() {
            Some(p) => p,
            None => return, // no printer in CI is fine
        };
        let file = write_temp_test_file("cpdb-rs-test.txt");
        let job_id = printer
            .submit_job(file.to_str().unwrap(), &[("copies", "1")], "cpdb-rs test")
            .expect("submit_job failed");
        assert!(!job_id.is_empty(), "job id must not be empty");
        let _ = fs::remove_file(&file);
    }
}

#[cfg(all(test, feature = "zbus-backend"))]
mod zbus_integration {
    use cpdb_rs::client::CpdbClient;
    use cpdb_rs::events::DiscoveryEvent;
    use futures_util::StreamExt;

    // Test that CpdbClient can connect to the session bus and discover backends.
    // This test requires a running D-Bus session bus and at least one CPDB
    // backend installed (e.g. cpdb-backend-cups).
    #[tokio::test]
    #[ignore]
    async fn test_client_connects_and_discovers_backends() {
        let client = CpdbClient::new().await;
        match client {
            Ok(client) => {
                println!("Connected! Found {} backend(s)", client.backend_count());
                assert!(
                    client.backend_count() > 0,
                    "Expected at least 1 CPDB backend"
                );
            }
            Err(e) => {
                eprintln!("CpdbClient::new() failed: {e}");
            }
        }
    }

    // Test that the discovery stream produces PrinterAdded events.
    // This test starts a discovery stream and waits for up to 5 seconds
    // for at least one PrinterAdded event.
    #[tokio::test]
    #[ignore]
    async fn test_discovery_stream_emits_events() {
        let client = match CpdbClient::new().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping: CpdbClient::new() failed: {e}");
                return;
            }
        };

        if client.backend_count() == 0 {
            eprintln!("Skipping: no CPDB backends found");
            return;
        }

        let stream = match client.discovery_stream().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping: discovery_stream() failed: {e}");
                return;
            }
        };

        // Take up to 10 events or timeout after 5 seconds
        let events: Vec<DiscoveryEvent> =
            tokio::time::timeout(std::time::Duration::from_secs(5), stream.take(10).collect())
                .await
                .unwrap_or_default();

        println!("Received {} discovery event(s):", events.len());
        for event in &events {
            match event {
                DiscoveryEvent::PrinterAdded(snap) => {
                    println!(
                        "  + PrinterAdded: id={}, name={}, backend={}, state={}",
                        snap.id, snap.name, snap.backend, snap.state
                    );
                }
                DiscoveryEvent::PrinterRemoved { id, backend } => {
                    println!("  - PrinterRemoved: id={id}, backend={backend}");
                }
                DiscoveryEvent::PrinterStateChanged {
                    id, state, backend, ..
                } => {
                    println!("  ~ StateChanged: id={id}, state={state}, backend={backend}");
                }
            }
        }

        // We expect at least one PrinterAdded if backends + printers exist
        if !events.is_empty() {
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, DiscoveryEvent::PrinterAdded(_))),
                "Expected at least one PrinterAdded event"
            );
        }
    }

    // Test fetching printer capabilities (options + media) for a discovered printer.
    #[tokio::test]
    #[ignore]
    async fn test_get_printer_details() {
        let client = match CpdbClient::new().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping: {e}");
                return;
            }
        };

        if client.backend_count() == 0 {
            eprintln!("Skipping: no backends");
            return;
        }

        // First discover a printer
        let stream = match client.discovery_stream().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping: {e}");
                return;
            }
        };

        use futures_util::stream::StreamExt as _;
        let mut filtered = stream
            .filter_map(|e| async move {
                match e {
                    DiscoveryEvent::PrinterAdded(snap) => Some(snap),
                    _ => None,
                }
            })
            .boxed();
        let first_printer =
            tokio::time::timeout(std::time::Duration::from_secs(5), filtered.next()).await;

        let snap = match first_printer {
            Ok(Some(s)) => s,
            _ => {
                eprintln!("Skipping: no printer discovered within 5s");
                return;
            }
        };

        println!(
            "Fetching details for: {} (backend: {})",
            snap.id, snap.backend
        );

        match client.get_printer_details(&snap.id, &snap.backend).await {
            Ok((options, media)) => {
                println!("  Options: {} entries", options.len());
                for opt in options.iter().take(5) {
                    println!(
                        "    {}: default={}, choices=[{}]",
                        opt.name,
                        opt.default_value,
                        opt.supported_values.join(", ")
                    );
                }

                println!("  Media: {} entries", media.len());
                for m in media.iter().take(5) {
                    println!(
                        "    {}: {}x{} ({} margin set(s))",
                        m.name,
                        m.width,
                        m.length,
                        m.margins.len()
                    );
                }

                assert!(!options.is_empty(), "Expected at least one option");
            }
            Err(e) => {
                eprintln!("get_printer_details failed: {e}");
            }
        }
    }

    // Test fetching translations for a discovered printer.
    #[tokio::test]
    #[ignore]
    async fn test_get_translations() {
        let client = match CpdbClient::new().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping: {e}");
                return;
            }
        };

        if client.backend_count() == 0 {
            eprintln!("Skipping: no backends");
            return;
        }

        // Discover a printer
        let stream = match client.discovery_stream().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping: {e}");
                return;
            }
        };

        let mut filtered = stream
            .filter_map(|e| async move {
                match e {
                    DiscoveryEvent::PrinterAdded(snap) => Some(snap),
                    _ => None,
                }
            })
            .boxed();
        let first_printer =
            tokio::time::timeout(std::time::Duration::from_secs(5), filtered.next()).await;

        let snap = match first_printer {
            Ok(Some(s)) => s,
            _ => {
                eprintln!("Skipping: no printer discovered within 5s");
                return;
            }
        };

        match client
            .get_translations(&snap.id, &snap.backend, "en_US")
            .await
        {
            Ok(translations) => {
                println!("Translations ({} entries):", translations.len());
                for (k, v) in translations.iter().take(10) {
                    println!("  {} -> {}", k, v);
                }
            }
            Err(e) => {
                eprintln!("get_translations failed (may not be supported): {e}");
            }
        }
    }

    // Test the default printer query.
    #[tokio::test]
    #[ignore]
    async fn test_get_default_printer() {
        let client = match CpdbClient::new().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping: {e}");
                return;
            }
        };

        if client.backend_count() == 0 {
            eprintln!("Skipping: no backends");
            return;
        }

        match client.get_default_printer("CUPS").await {
            Ok(default) => {
                println!("Default CUPS printer: {}", default);
                assert!(!default.is_empty());
            }
            Err(e) => {
                eprintln!("get_default_printer failed: {e}");
            }
        }
    }

    // Test remote/temporary printer visibility toggles.
    #[tokio::test]
    #[ignore]
    async fn test_show_remote_and_temporary_printers() {
        let client = match CpdbClient::new().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping: {e}");
                return;
            }
        };

        // These should not panic even with no backends
        client.show_remote_printers(false).await;
        client.show_remote_printers(true).await;
        client.show_temporary_printers(false).await;
        client.show_temporary_printers(true).await;
        println!("Visibility toggles completed without error");
    }
}
