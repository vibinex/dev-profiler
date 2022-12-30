use crate::writer::OutputWriter;
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
pub struct ErrorInfo {
    errors: Vec<String>
}

impl ErrorInfo {
    pub fn new() -> Self {
        Self{ errors: Vec::<String>::new()}
    }
    pub fn push(&mut self, estr: &str) {
        self.errors.push(estr.to_string());
    }
    pub fn write_err(&self, writer: &mut OutputWriter) -> Result<(), std::io::Error>{
        let serialized = serde_json::to_string(&self).unwrap_or_default();
        writer.writeln(serialized.as_str().as_ref())
    }
}