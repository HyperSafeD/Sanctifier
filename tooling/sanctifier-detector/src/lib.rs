pub mod config;
pub mod events;
pub mod rules;
pub mod service;
pub mod webhook;

pub use config::{DetectorConfig, HourWindow};
pub use events::CallRecord;
pub use rules::{Alert, AlertSeverity, DetectionRule};
pub use service::DetectorService;
