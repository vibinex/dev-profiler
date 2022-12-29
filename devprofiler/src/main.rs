use clap::Parser;
// mod analyzer;
// use crate::analyzer::RepoAnalyzer;
mod scanner;
use crate::scanner::RepoScanner;

// TODO - logging
// TODO - error handling

#[derive(Parser)]
struct Cli {
    /// path
    path: std::path::PathBuf,
}

fn main() {
    let args = Cli::parse();
    let rscanner = RepoScanner::new(args.path);
    let (pathvec, patherrs) = rscanner.scan();
    println!("{:?}", pathvec);
    println!("{:?}", patherrs);
    // let ranalyzer = RepoAnalyzer::new(args.path, args.user_email.clone());
    // ranalyzer.analyze()
}