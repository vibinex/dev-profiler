use clap::Parser;
mod analyzer;
use crate::analyzer::RepoAnalyzer;
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
    // let rscanner = RepoScanner::new(args.path);
    // let (pathvec, patherrs) = rscanner.scan();
    // println!("{:?}", pathvec);
    // println!("{:?}", patherrs);
    let ranalyzer = RepoAnalyzer::new(args.path);
    if ranalyzer.is_some() {
        let res = ranalyzer.expect("Checked, is not none").analyze();
        println!("{:?}", res.unwrap());
    }
}