use crate::writer::OutputWriter;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Default)]
pub struct RuntimeInfo {
    errors: Vec<String>,
    version: String,
    timestamp: u64,
    logs: Vec<String>,
}

impl RuntimeInfo {
    pub fn new() -> Self {
        let since_the_epoch = SystemTime::now().duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self{ 
            errors: Vec::<String>::new(),
            version: option_env!("CARGO_PKG_VERSION").unwrap_or("unknown")
                .to_string(),
            timestamp: since_the_epoch.as_secs() * 1000 +
                since_the_epoch.subsec_nanos() as u64 / 1_000_000,
            logs: Vec::<String>::new(),
        }
    }
    pub fn record_err(&mut self, estr: &str) {
        self.errors.push(estr.to_string());
    }
    pub fn write_runtime_info(&self, writer: &mut OutputWriter) -> Result<(), std::io::Error>{
        let serialized = serde_json::to_string(&self).unwrap_or_default();
        writer.writeln(serialized.as_str().as_ref())
    }
}