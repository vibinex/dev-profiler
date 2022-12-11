use workerpool::Pool;
use workerpool::thunk::{Thunk, ThunkWorker};
use git2::{Commit, Repository};
use std::sync::mpsc::channel;
use std::sync::Mutex;
use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// author name or pattern
    author: String,
    /// repository path
    path: std::path::PathBuf,
}

#[derive(Clone)]
struct CommitInfo {
    author: String,
    // commit_id: String,
}

impl CommitInfo {
    fn new(commit: Commit) -> Self {
        Self {
            author: commit.author().to_string(),
            // commit_id: commit.id().to_string(),
        }
    }
}

unsafe impl Send for CommitInfo {}

fn map_metrics(cinfo: &CommitInfo) -> String {
    cinfo.clone().author
}

fn analyze_repo(arg_ref: &Cli) {
    let repo = Repository::discover(&arg_ref.path).unwrap();
        let mut revwalk = repo.revwalk().unwrap();
        let mut count = 0;
        let n_workers = 4;
        let pool = Pool::<ThunkWorker<String>>::new(n_workers);
        let (tx, rx) = channel();
        revwalk.push_head().unwrap();
        // revwalk.set_sorting(git2::Sort::TIME)?;
        for rev in revwalk {
            let commit = repo.find_commit(rev.unwrap()).unwrap();
            // let message = commit.summary_bytes().unwrap_or_else(|| commit.message_bytes());
            let author = commit.author();
            if author.to_string().contains(&(arg_ref.author)) {
                count = count + 1;
                // let ncommit = commit.clone();
                let arc_commit = Mutex::new(CommitInfo::new(commit.clone()));
                pool.execute_to(tx.clone(), Thunk::of(move || -> String {
                    let refcommit = arc_commit.lock().unwrap();
                    map_metrics(&refcommit)
                }));
            }
        }
        drop(tx);
        println!("Number of commits: {}", count);
        for r in rx.iter() {
            println!("rx = {:?}", r);
        }
}


fn main() {
    let args = Cli::parse();
    analyze_repo(&args);
}


// fn answer() -> i64 {
//     42
// }
// #[test]
// // test all branches are there
// // test filtering of commits, if all commits visited and filtering is correct
// // test correctness of stats
// // test reduce step correctness
// // test final calculation
// // test jsonl rendering for correctness
// // test thread management? if all are finished etc
// fn check_answer_validity() {
//     assert_eq!(answer(), 42);
// }