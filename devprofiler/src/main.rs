#![allow(unused)]

use workerpool::Pool;
use workerpool::thunk::{Thunk, ThunkWorker};
use git2::{Commit, ObjectType, Repository};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
// Search for a pattern in a file and display the lines that contain it.
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

unsafe impl Send for Commit {}

fn map_metrics(commit: &Commit) -> String {
    commit.id().to_string()
}

fn some_commit() -> Result<i32, Box<git2::Error>> {
    let repo = Repository::discover("/home/tapishr/mentorship-website")?;
        let mut revwalk = repo.revwalk()?;
        let mut count = 0;
        let n_workers = 4;
        let pool = Pool::<ThunkWorker<String>>::new(n_workers);
        let (tx, rx) = channel();
        revwalk.push_head()?;
        // revwalk.set_sorting(git2::Sort::TIME)?;
        for rev in revwalk {
            let commit = repo.find_commit(rev?)?;
            let message = commit.summary_bytes().unwrap_or_else(|| commit.message_bytes());
            let author = commit.author();
            let mut_commit = Arc::new(Mutex::new(commit));
            if author.to_string().contains("tapish") {
                count = count + 1;
                pool.execute_to(tx.clone(), Thunk::of(move || -> String {
                    let refcommit = mut_commit.lock().unwrap();
                    map_metrics(&refcommit)
                }));
            }
        }
        println!("Number of commits: {}", count);
        let threads = rx.iter();
        Ok(0)
}


fn main() {
    // let args = Cli::parse();
    let res = some_commit();
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