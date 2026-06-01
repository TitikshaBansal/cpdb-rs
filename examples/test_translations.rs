//! Example: Fetch localized translations for a printer's options.

use cpdb_rs::CpdbClient;

#[tokio::main]
async fn main() -> cpdb_rs::Result<()> {
    let client = CpdbClient::new().await?;

    let printers = client.get_all_printers().await?;
    let printer = match printers.iter().find(|p| p.is_ready()) {
        Some(p) => p,
        None => {
            eprintln!("No ready printers found.");
            return Ok(());
        }
    };

    println!("Printer: {} [{}]\n", printer.name, printer.id);

    // Fetch options first (so we know what to translate)
    let (options, _media) = client
        .get_printer_details(&printer.id, &printer.backend)
        .await?;
    println!("Options ({} total):", options.len());
    for opt in options.iter().take(5) {
        println!("  {}: default='{}'", opt.name, opt.default_value);
    }

    // Fetch translations
    // Try the system locale, falling back to en_US
    let locale = std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
    // CPDB expects just the language_COUNTRY part (e.g. "en_US")
    let locale_short = locale.split('.').next().unwrap_or("en_US");

    println!("\n=== Translations (locale: {}) ===", locale_short);
    match client
        .get_translations(&printer.id, &printer.backend, locale_short)
        .await
    {
        Ok(translations) => {
            if translations.is_empty() {
                println!("  (no translations returned - backend may not support this locale)");
            } else {
                // Sort for consistent output
                let mut entries: Vec<_> = translations.iter().collect();
                entries.sort_by_key(|(k, _)| (*k).clone());

                println!("  {} entries:", entries.len());
                for (key, label) in &entries {
                    println!("  {} -> {}", key, label);
                }
            }
        }
        Err(e) => {
            eprintln!("  Error: {e}");
        }
    }

    Ok(())
}
