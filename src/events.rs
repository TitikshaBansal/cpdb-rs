//! Discovery events emitted by CPDB backends.
//!
//! These map directly to the D-Bus signals defined in the
//! `org.openprinting.PrintBackend` interface.

/// An event emitted during printer discovery or state monitoring.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A printer was discovered or re-announced.
    PrinterAdded(PrinterSnapshot),
    /// A printer was removed from the backend.
    PrinterRemoved { id: String, backend: String },
    /// A printer's state or accepting-jobs status changed.
    PrinterStateChanged {
        id: String,
        backend: String,
        state: String,
        accepting_jobs: bool,
    },
}

/// Snapshot of a printer's identity and status at a point in time.
#[derive(Debug, Clone)]
pub struct PrinterSnapshot {
    pub id: String,
    pub name: String,
    pub info: String,
    pub location: String,
    pub make_model: String,
    pub state: String,
    pub accepting_jobs: bool,
    pub backend: String,
}

impl PrinterSnapshot {
    pub fn is_ready(&self) -> bool {
        self.state == "idle" && self.accepting_jobs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> PrinterSnapshot {
        PrinterSnapshot {
            id: "HP-LaserJet-Pro".to_string(),
            name: "HP LaserJet Pro".to_string(),
            info: "Office printer".to_string(),
            location: "Room 42".to_string(),
            make_model: "HP LaserJet Pro MFP".to_string(),
            state: "idle".to_string(),
            accepting_jobs: true,
            backend: "CUPS".to_string(),
        }
    }

    #[test]
    fn snapshot_is_ready_when_idle_and_accepting() {
        let snap = sample_snapshot();
        assert!(snap.is_ready());
    }

    #[test]
    fn snapshot_not_ready_when_busy() {
        let mut snap = sample_snapshot();
        snap.state = "printing".to_string();
        assert!(!snap.is_ready());
    }

    #[test]
    fn snapshot_not_ready_when_not_accepting() {
        let mut snap = sample_snapshot();
        snap.accepting_jobs = false;
        assert!(!snap.is_ready());
    }

    #[test]
    fn snapshot_clones_correctly() {
        let snap = sample_snapshot();
        let clone = snap.clone();
        assert_eq!(snap.id, clone.id);
        assert_eq!(snap.backend, clone.backend);
    }

    #[test]
    fn discovery_event_printer_added() {
        let event = DiscoveryEvent::PrinterAdded(sample_snapshot());
        assert!(matches!(event, DiscoveryEvent::PrinterAdded(_)));
    }

    #[test]
    fn discovery_event_printer_removed() {
        let event = DiscoveryEvent::PrinterRemoved {
            id: "HP-123".to_string(),
            backend: "CUPS".to_string(),
        };
        match &event {
            DiscoveryEvent::PrinterRemoved { id, backend } => {
                assert_eq!(id, "HP-123");
                assert_eq!(backend, "CUPS");
            }
            _ => panic!("Expected PrinterRemoved"),
        }
    }

    #[test]
    fn discovery_event_state_changed() {
        let event = DiscoveryEvent::PrinterStateChanged {
            id: "HP-123".to_string(),
            backend: "CUPS".to_string(),
            state: "printing".to_string(),
            accepting_jobs: true,
        };
        match &event {
            DiscoveryEvent::PrinterStateChanged {
                state,
                accepting_jobs,
                ..
            } => {
                assert_eq!(state, "printing");
                assert!(*accepting_jobs);
            }
            _ => panic!("Expected PrinterStateChanged"),
        }
    }

    #[test]
    fn events_are_clone() {
        let event = DiscoveryEvent::PrinterAdded(sample_snapshot());
        let clone = event.clone();
        assert!(matches!(clone, DiscoveryEvent::PrinterAdded(_)));
    }
}
