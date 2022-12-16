use clap::Parser;
use git2::{ Repository, Diff, Commit };
use detect_lang;
use std::path::Path;
use serde::Serialize;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

#[derive(Parser)]
struct Cli {
    /// user name/email or pattern
    user: String,
    /// repository path
    path: std::path::PathBuf,
}

#[derive(Clone, Debug, Serialize)]
struct DiffInfo {
    insertions: usize,
    deletions: usize,
    files_changed: usize,
    file_info: Vec<DiffFileInfo>,
}

#[derive(Clone, Debug, Serialize)]
struct DiffFileInfo {
    filepath: String,
    v_language: String,
}

#[derive(Clone, Debug, Serialize)]
struct CommitInfo {
    commit_id: String,
    author_name: String,
    author_email: String,
    ts_secs: i64,
    ts_offset_mins: i64,
    parents: Vec<String>,
    diff_info: DiffInfo,
}

impl CommitInfo {
    fn new(commit: &Commit, diff: &Diff) -> Self {
        let tsecs = commit.time().seconds();
        let toffset :i64 = commit.time().offset_minutes().into();
        let mut cparents :Vec<String>  = Vec::new();
        for c in commit.parents() {
            cparents.push(c.id().to_string());
        }
        Self {
            commit_id: commit.id().to_string(),
            author_name: commit.author().name().unwrap().to_string(),
            author_email:commit.author().email().unwrap().to_string(),
            ts_secs: tsecs,
            ts_offset_mins: toffset,
            parents: cparents,
            diff_info: Self::get_diffs(diff),
        }
    }
    
    fn get_diffs(diff: &Diff) -> DiffInfo{
        
        let mut diffvec: Vec<DiffFileInfo> = Vec::new();
        for delta in diff.deltas() {
            let fpath = delta.new_file().path();
            if fpath.is_none() {
                eprintln!("No filepath");
                continue;
            }
            let filepath = fpath.unwrap();
            let lang = match detect_lang::from_path(filepath) {
                Some(langid) => langid.id().to_string(),
                None => "None".to_string(),
            };
            diffvec.push(DiffFileInfo::new(&filepath, &lang));
        }
        let stats = diff.stats().unwrap();
        DiffInfo {
            insertions: stats.insertions(),
            deletions: stats.deletions(),
            files_changed: stats.files_changed(),
            file_info: diffvec,
        }
    }
}

impl DiffFileInfo {
    fn new(path: &Path, lang: &String) -> Self {
        Self {
            filepath: path.to_path_buf().into_os_string().into_string().unwrap(),
            v_language: String::from(lang),
        }
    }
}

fn analyze_repo(arg_ref: &Cli) {
    let repo = Repository::discover(&arg_ref.path).unwrap();
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();
    let file = File::create("devprofiler.json.gz").unwrap();
    let mut bufw = BufWriter::new(file);
    let mut gze = GzEncoder::new(bufw, Compression::default());
    for rev in revwalk {
        let commit = repo.find_commit(rev.unwrap()).unwrap();
        let commit_tree = commit.tree().unwrap();
	    let parent = commit.parent(0);
        if !parent.is_ok() {
            continue;
            // todo - fix me
        }
        let parent_tree = parent.unwrap().tree().unwrap();
        let diff = repo.diff_tree_to_tree(Some(&commit_tree), Some(&parent_tree), None).unwrap();
        let cinfo = CommitInfo::new(&commit, &diff);
        let serialized = serde_json::to_string(&cinfo).unwrap();
        println!("serialized = {}", serialized);
        gze.write(serialized.as_bytes());
    }
    gze.finish();
}

fn main() {
    let args = Cli::parse();
    analyze_repo(&args);
}