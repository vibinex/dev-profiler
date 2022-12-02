#![allow(unused)]

// use clap::Parser;
// use anyhow::{Context, Result};
// use std::fs::File;
use walkdir::{WalkDir, DirEntry};
use std::path::{Path, PathBuf};
use std::fs;
use time;
use git2::{Commit, ObjectType, Repository};
/// Search for a pattern in a file and display the lines that contain it.
// #[derive(Parser)]
// struct Cli {
//     /// The pattern to look for
//     pattern: String,
//     /// The path to the file to read
//     path: std::path::PathBuf, // PathBuf is like a String but for file system paths that work cross-platform.
// }

// fn is_git(entry: &DirEntry) -> bool {
//     entry.file_name().to_str().map(|s| s == ".git").unwrap_or(false)
// }
fn is_hidden(entry: &DirEntry) -> bool {
    entry
         .file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

fn map_metrics(commit: &Commit) {
    let timestamp = commit.time().seconds();
    let tm = time::at(time::Timespec::new(timestamp, 0));
    println!("commit {}\nAuthor: {}\nDate:   {}\n\n    {}",
             commit.id(),
             commit.author(),
             tm.rfc822(),
             commit.message().unwrap_or("no commit message"));
}

fn some_commit() -> Result<i32, Box<git2::Error>> {
    let repo = Repository::discover("/Users/tapishpersonal/Code/mentorship-website")?;
        let mut revwalk = repo.revwalk()?;
        let mut count = 0;
        revwalk.push_head()?;
        // revwalk.set_sorting(git2::Sort::TIME)?;
        for rev in revwalk {
            let commit = repo.find_commit(rev?)?;
            let message = commit.summary_bytes().unwrap_or_else(|| commit.message_bytes());
            // let t = commit.time();
            let author = commit.author();
            if author.to_string().contains("tapish") {
                println!("{}\t{}\t{}", commit.id(), author, String::from_utf8_lossy(message));
                count = count + 1;
            }
        }
        println!("Number of commits: {}", count);
        Ok(0)
    // let repo = match Repository::open("/Users/tapishpersonal/Code/mentorship-website") {
    //     Ok(repo) => repo,
    //     Err(e) => panic!("failed to open: {}", e),
    // };
    // let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    // obj.into_commit().map_err(|_| git2::Error::from_str("Couldn't find commit"))
}


fn main() {
    // let args = Cli::parse();
    let repo_path = [Path::new("/Users/tapishpersonal/Code/mentorship-website")];
    let aliases = [""];
    for repo in repo_path {
        // get all branches
        let mut refheads = repo.to_path_buf();
        refheads.push(".git");
        refheads.push("refs");
        refheads.push("heads");
        let mut heads = Vec::new();
        // iterate over dirs and files
        let mut packedrefs = repo.to_path_buf();
        packedrefs.push(".git");
        packedrefs.push("packed-refs");
        let mut content = fs::read_to_string(packedrefs)
                .expect("Unable to read file");
        println!("{}", content);
        let walker = WalkDir::new(refheads).into_iter(); // TODO - do correct error handling here
        for entry in walker.filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && !is_hidden(e)) { // is_hidden to avoid files like .DS_Store
            // files are read and commits stored in lists
            let mut commit = fs::read_to_string(entry.path())
                .expect("Unable to read file");
            commit.pop(); // remove trailing newline
            heads.push(commit);
        }
        println!("{:?}", heads);

        // open some branch file and go over all commits, filter by aliases
        // for each commit, get stats -> in worker threads
    }
    let res = some_commit();
    // map_metrics(obj);

    // wait for all threads to finish
    // do the reduce step for each metric (like no of years in each lang) in diff thread
    // wait for threads to finish
    // render jsonl

    // Note: See that .expect method here? This is a shortcut function to quit that will make the program exit immediately when the value (in this case the input file) could not be read. It’s not very pretty, and in the next chapter on Nicer error reporting we will look at how to improve this.
    // let content = std::fs::read_to_string(&args.path)
    //     .with_context(|| format!("could not read file `{}`", &args.path.display()))?;
    // This is not the best implementation: It will read the whole file into memory – however large the file may be. Find a way to optimize it! (One idea might be to use a BufReader instead of read_to_string()
    // for line in content.lines() {
    //     if line.contains(&args.pattern) {
    //         println!("{}", line);
    //     }
    // }

    //Scanner
    // let walker = WalkDir::new("/").into_iter();
    // for entry in walker.filter_map(Result::ok).filter(|e| is_git(e)) {
        // Also see filter_entry
    //     println!("{}", entry.path().display());
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
// test all branches are there
// test filtering of commits, if all commits visited and filtering is correct
// test correctness of stats
// test reduce step correctness
// test final calculation
// test jsonl rendering for correctness
// test thread management? if all are finished etc
fn check_answer_validity() {
    assert_eq!(answer(), 42);
}