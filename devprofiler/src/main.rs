use clap::Parser;
mod analyzer;
use crate::analyzer::RepoAnalyzer;
mod writer;
use crate::writer::OutputWriter;
// use std::path::Path;
// mod scanner;
// use crate::scanner::RepoScanner;

// TODO - logging
// TODO - error handling

#[derive(Parser)]
struct Cli {
    /// path
    path: std::path::PathBuf,
}

// fn main() {
//     let path = Path::new("/tmp");
//     let parent = path.parent();
//     println!("{:?}", parent);
// }
fn main() {
    let args = Cli::parse();
    let writer_result = OutputWriter::new();
    if writer_result.is_ok() {
        let writer = &mut writer_result.expect("Checked, is ok");
        let ranalyzer_res = RepoAnalyzer::new(args.path);
        if ranalyzer_res.is_some() {
            let ranalyzer = ranalyzer_res.expect("Checked, is not none");
            let res = ranalyzer.analyze(writer);
            println!("{:?}", res.unwrap());
        }
        writer.finish();
    }
    // let rscanner = RepoScanner::new(args.path);
    // let (pathvec, patherrs) = rscanner.scan();
    // println!("{:?}", pathvec);
    // println!("{:?}", patherrs);
}