#![allow(unused)]

// use clap::Parser;
// use anyhow::{Context, Result};
// use std::fs::File;
use std::path::{Path, PathBuf};
use std::fs;
// use time;
use git2::{Commit, ObjectType, Repository, Diff, Tree, Error, Oid};
use detect_lang::from_path;
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

fn sort_commit_language_wise(repo: &Repository, author_name: &str, mut count: i32, commit: &Commit) -> i32 {
	// let message = commit.summary_bytes().unwrap_or_else(|| commit.message_bytes());
	let commit_timesatmp = commit.time();
	let commit_tree = commit.tree().unwrap();
	let author = commit.author();
	if author.to_string().contains(author_name) {
		// println!("{}\t{}\t{}", commit.id(), author, String::from_utf8_lossy(message));
		let parent_commit = commit.parent(0).unwrap();
		let parent_tree = parent_commit.tree().unwrap();

		let language_list: Vec<&str> = Vec::new();

		let diff = repo.diff_tree_to_tree(Some(&commit_tree), Some(&parent_tree), None).unwrap();
		for delta in diff.deltas() {
			// println!("{:?}", delta.new_file().path().unwrap());
			let detected_language = detect_lang::from_path(delta.new_file().path().unwrap());
			println!("{:?}", detected_language);
			// if language_list.contains()

		}
		count = count + 1;

	}
	return count;

}

// fn is_git(entry: &DirEntry) -> bool {
//     entry.file_name().to_str().map(|s| s == ".git").unwrap_or(false)
// }

fn some_commit() -> Result<i32, Box<git2::Error>> {
    let repo = Repository::discover("/home/muskanp/mentorship-website").expect("error occured");
	let mut revwalk = repo.revwalk().expect("revwalk failed");
	revwalk.push_head().expect("push head failed");
	revwalk.set_sorting(git2::Sort::TIME).expect("sorting failed");
	let mut author_name = "muskan";
	let mut count = 0;

	for rev in revwalk {

		let commit = repo.find_commit(rev?)?;
		count = sort_commit_language_wise(&repo, author_name, count, &commit);
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