use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::io::BufWriter;
use flate2::Compression;
use flate2::write::GzEncoder;

pub struct OutputWriter {
    writer: GzEncoder<BufWriter<File>>
}

impl OutputWriter {
    pub fn new() -> Result<OutputWriter, Error>{
        let file = File::create("devprofile.jsonl.gz")?;
        let bufw = BufWriter::new(file);
        let gze = GzEncoder::new(bufw, Compression::default());
        Ok(Self{writer: gze})
    }

    pub fn writeln(&mut self, line: &str) -> Result<(), Error>{
        writeln!(self.writer, "{}", line.to_string())
    }

    pub fn finish(&mut self) -> Result<(), Error>{
        self.writer.try_finish()
    }
}