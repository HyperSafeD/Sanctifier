use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CallRecord {
    #[serde(default)]
    pub id: String,
    pub contract_id: String,
    pub function: String,
    pub caller: String,
    pub success: bool,
    pub timestamp_unix: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum EventFeed {
    Records(Vec<CallRecord>),
    Envelope { records: Vec<CallRecord> },
}

impl CallRecord {
    pub fn stable_key(&self) -> String {
        if self.id.is_empty() {
            format!(
                "{}:{}:{}:{}",
                self.contract_id, self.function, self.caller, self.timestamp_unix
            )
        } else {
            self.id.clone()
        }
    }
}
