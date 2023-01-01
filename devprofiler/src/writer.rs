use std::fs::File;
use std::io::Error;
use std::io::Write;
use std::io::BufWriter;
use flate2::Compression;
use flate2::write::GzEncoder;

pub struct OutputWriter {
    writer: GzEncoder<BufWriter<File>>,
    iowriter: Option<BufWriter<File>>
}

impl OutputWriter {
    pub fn new() -> Result<OutputWriter, Error>{
        let file = File::create("devprofile.jsonl.gz")?;
        let bufw = BufWriter::new(file);
        let gze = GzEncoder::new(bufw, Compression::default());
        Ok(Self{
            writer: gze,
            iowriter: None,
        })
    }

    pub fn writeln(&mut self, line: &str) -> Result<(), Error>{
        writeln!(self.writer, "{}", line.to_string())
    }

    pub fn write_io_err(&mut self, line: &str) -> Result<(), Error>{
        if self.iowriter.is_none() {
            match self.create_io_file_writer() {
                Ok(io_writer) => {self.iowriter = io_writer;},
                Err(error) => { return Err(error); },
            }
            
        }
        let writer_borrow = self.iowriter.as_mut().expect("Checked, is some");
        writeln!(writer_borrow, "{}", line.to_string())
    }

    pub fn finish(&mut self) -> Result<(), Error>{
        if self.iowriter.is_some() {
            let writer_borrow = self.iowriter.as_mut().expect("Checked, is some");
            writer_borrow.flush()?;
        }
        self.writer.try_finish()
    }

    fn create_io_file_writer(&self) -> Result<Option<BufWriter<File>>, Error>{
        let iofile = File::create("io_errors.txt")?;
        Ok(Some(BufWriter::new(iofile)))
    }
}