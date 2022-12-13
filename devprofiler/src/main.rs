use workerpool::Pool;
use workerpool::thunk::{Thunk, ThunkWorker};
use git2::{Commit, Repository};
use std::sync::mpsc::channel;
use std::sync::Mutex;
use clap::Parser;

use core::time;
use std::hash::Hash;
// use clap::Parser;
// use anyhow::{Context, Result};
// use std::fs::File;
use std::path::{Path, PathBuf};
use std::fs;
use git2::{Commit, ObjectType, Repository, Diff, Tree, Error, Oid};
use detect_lang::{from_path, Language};
use std::collections::HashMap;
/// Search for a pattern in a file and display the lines that contain it.
// #[derive(Parser)]
// struct Cli {
//     /// The pattern to look for
//     pattern: String,
//     /// The path to the file to read
//     path: std::path::PathBuf, // PathBuf is like a String but for file system paths that work cross-platform.
// }

// Used this method for debugging only.
fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>());
}

fn sort_commit_language_wise(repo: &Repository, commit: &Commit, parent_commit: &Commit, mut language_wise_commits: HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
	let commit_tree = commit.tree().unwrap();
	let parent_tree = parent_commit.tree().unwrap();
	let mut commit_language_list: Vec<&str> = Vec::new();

	let diff = repo.diff_tree_to_tree(Some(&commit_tree), Some(&parent_tree), None).unwrap();
	for delta in diff.deltas() {
		let detected_language = detect_lang::from_path(delta.new_file().path().unwrap());
		match detected_language {
			Some(Language(name, extension)) => {
				if !commit_language_list.contains(&name){
					commit_language_list.push(name);
				}
			}
			None => {
				// Handle the case where the option is None
			}
		}
	}
	for language in commit_language_list {
		if language_wise_commits.contains_key(language) {
			let commit_timestamps = language_wise_commits.get_mut(language).unwrap();
			commit_timestamps.push(commit.time().seconds().to_string());
		} else {
			language_wise_commits.insert(language.to_string(), vec![commit.time().seconds().to_string()]);
		}
	}

	return language_wise_commits;

}

// fn is_git(entry: &DirEntry) -> bool {
//     entry.file_name().to_str().map(|s| s == ".git").unwrap_or(false)
// }

fn some_commit() -> Result<i32, Box<git2::Error>> {
    let repo = Repository::discover("/home/muskanp/mentorship-website").expect("error occured");
	let mut revwalk = repo.revwalk().expect("revwalk failed");
	let mut author_name = "muskan";
	let mut count = 0;
	let mut language_wise_commits: HashMap<String, Vec<String>> = HashMap::new();
	
	revwalk.push_head().expect("push head failed");
	revwalk.set_sorting(git2::Sort::TIME).expect("sorting failed");

	for rev in revwalk {

		let commit = repo.find_commit(rev?)?;
		let author = commit.author();
		if author.to_string().contains(author_name) {
			let parent_commit = commit.parent(0).unwrap(); //We are only passing the recent parent of a commit, but we will have to do it for all the parents of a commit.
			language_wise_commits = sort_commit_language_wise(&repo, &commit, &parent_commit, language_wise_commits);
		}	
	}
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