use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Saved user preferences. Serializable, no I/O.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PrinterConfig {
    /// Last-used printer identifier
    pub last_printer_id: Option<String>,
    /// Last-used backend name
    pub last_backend: Option<String>,
    /// Last-used settings (copies, media, sides, etc.)
    pub settings: HashMap<String, String>,
}

impl PrinterConfig {
    /// Creates a new config capturing the current dialog state.
    pub fn new(printer_id: &str, backend: &str, settings: HashMap<String, String>) -> Self {
        Self {
            last_printer_id: Some(printer_id.to_string()),
            last_backend: Some(backend.to_string()),
            settings,
        }
    }

    /// Returns `true` if this config has a saved printer.
    pub fn has_printer(&self) -> bool {
        self.last_printer_id.is_some()
    }

    /// Gets a saved setting value by key.
    pub fn get_setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        let config = PrinterConfig::default();
        assert!(config.last_printer_id.is_none());
        assert!(config.last_backend.is_none());
        assert!(config.settings.is_empty());
        assert!(!config.has_printer());
    }

    #[test]
    fn new_config_captures_state() {
        let mut settings = HashMap::new();
        settings.insert("copies".to_string(), "2".to_string());
        settings.insert("media".to_string(), "iso_a4_210x297mm".to_string());

        let config = PrinterConfig::new("HP-123", "CUPS", settings);
        assert_eq!(config.last_printer_id.as_deref(), Some("HP-123"));
        assert_eq!(config.last_backend.as_deref(), Some("CUPS"));
        assert!(config.has_printer());
        assert_eq!(config.get_setting("copies"), Some("2"));
        assert_eq!(config.get_setting("media"), Some("iso_a4_210x297mm"));
    }

    #[test]
    fn get_setting_returns_none_for_missing() {
        let config = PrinterConfig::default();
        assert!(config.get_setting("nonexistent").is_none());
    }

    #[test]
    fn serde_roundtrip_json() {
        let mut settings = HashMap::new();
        settings.insert("copies".to_string(), "3".to_string());
        settings.insert("sides".to_string(), "two-sided-long-edge".to_string());

        let config = PrinterConfig::new("Epson-ET-2850", "CUPS", settings);
        let json = serde_json::to_string(&config).unwrap();
        let loaded: PrinterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn serde_roundtrip_default() {
        let config = PrinterConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let loaded: PrinterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn config_is_clone() {
        let config = PrinterConfig::new("test", "CUPS", HashMap::new());
        let clone = config.clone();
        assert_eq!(config, clone);
    }
}
