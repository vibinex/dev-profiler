#![allow(unused)]

// use clap::Parser;
// use anyhow::{Context, Result};
// use std::fs::File;
use walkdir::{DirEntry, WalkDir};
/// Search for a pattern in a file and display the lines that contain it.
// #[derive(Parser)]
// struct Cli {
//     /// The pattern to look for
//     pattern: String,
//     /// The path to the file to read
//     path: std::path::PathBuf, // PathBuf is like a String but for file system paths that work cross-platform.
// }

fn is_git(entry: &DirEntry) -> bool {
    entry.file_name().to_str().map(|s| s == ".git").unwrap_or(false)
}

fn main() {
    // let args = Cli::parse();
    // Note: See that .expect method here? This is a shortcut function to quit that will make the program exit immediately when the value (in this case the input file) could not be read. It’s not very pretty, and in the next chapter on Nicer error reporting we will look at how to improve this.
    // let content = std::fs::read_to_string(&args.path)
    //     .with_context(|| format!("could not read file `{}`", &args.path.display()))?;
    // This is not the best implementation: It will read the whole file into memory – however large the file may be. Find a way to optimize it! (One idea might be to use a BufReader instead of read_to_string()
    // for line in content.lines() {
    //     if line.contains(&args.pattern) {
    //         println!("{}", line);
    //     }
    // }
    let walker = WalkDir::new("/").into_iter();
    for entry in walker.filter_map(Result::ok).filter(|e| is_git(e)) {
        println!("{}", entry.path().display());
    }
    // for entry in glob("/System/Volumes/Data/Users/tapishpersonal/**/.git").expect("Failed to read glob pattern") {
    //     if let Ok(path) = entry {
    //         println!("{:?}", path.display());
    //     }
    //     // match entry {
    //     //     Ok(path) => println!("{:?}", path.display()),
    //     //     // Err(e) => println!("{:?}", e),
    //     // }
    // }

    // Ok(())
    // let mut f = File::open("/Users/tapishpersonal/Code/mentorship-website/.git/objects/bb/721ad95595bd8e023387e7af46aef3480c80f3")?;
    // let metadata = f.metadata()?;
    // for (key, value) in metadata {
    //     println!("{}: {}", key, value);
    // }
    // Ok(())
}
fn answer() -> i64 {
    (42)
}
#[test]
fn check_answer_validity() {
    assert_eq!(answer(), 42);
}