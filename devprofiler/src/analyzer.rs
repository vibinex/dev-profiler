use git2::{ Repository, Diff, Commit };
use detect_lang;
use std::path::Path;
use serde::Serialize;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use sha256::digest;
use std::ffi::OsStr;
use std::path::PathBuf;

pub struct RepoAnalyzer {
    path: PathBuf,
    email: String
}

impl RepoAnalyzer {
    pub fn new(path: PathBuf, email: String) -> Self{
        Self { path, email }
    }

    pub fn analyze(&self) {
        let errinfo = ErrorInfo {errors: Vec::new()};
        let repo = Repository::discover(&self.path).unwrap();
        let repodir = self.path.as_path();
        let reponame = repodir.strip_prefix(repodir.parent().unwrap()).unwrap().as_os_str().to_str().unwrap_or_default().to_string();
        let mut revwalk = repo.revwalk().unwrap();
        revwalk.push_head().unwrap();
        let file = File::create("devprofile.jsonl.gz").unwrap();
        let bufw = BufWriter::new(file);
        let mut gze = GzEncoder::new(bufw, Compression::default());
        let rinfo: RunInfo = RunInfo {
            aliases: vec![self.email.clone()],
            repos: vec![self.path.as_os_str().to_str().unwrap_or_default().to_string().clone()],
        };
        let res1 = writeln!(gze, "{}", serde_json::to_string(&errinfo).unwrap());
        let res2 = writeln!(gze, "{}", serde_json::to_string(&rinfo).unwrap());
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
            let cinfo = CommitInfo::new(&commit, &diff, reponame.clone());
            let serialized = serde_json::to_string(&cinfo).unwrap();
            let res = writeln!(gze, "{}", serialized);
        }
        let result = gze.finish();
    }
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
    path_hash: String,
    filename: String,
    v_language: String,
}

#[derive(Clone, Debug, Serialize)]
struct CommitInfo {
    commit_id: String,
    repo_name: String,
    author_name: String,
    author_email: String,
    ts_secs: i64,
    ts_offset_mins: i64,
    parents: Vec<String>,
    diff_info: DiffInfo,
}

impl CommitInfo {
    fn new(commit: &Commit, diff: &Diff, reponame: String) -> Self {
        let tsecs = commit.time().seconds();
        let toffset :i64 = commit.time().offset_minutes().into();
        let mut cparents :Vec<String>  = Vec::new();
        for c in commit.parents() {
            cparents.push(digest(c.id().to_string()));
        }
        Self {
            commit_id: digest(commit.id().to_string()),
            repo_name: reponame,
            author_name: digest(commit.author().name().unwrap().to_string()),
            author_email: digest(commit.author().email().unwrap().to_string()),
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
        let fname = digest(path.file_stem().unwrap().to_str().unwrap().to_string());
        let extension = path.extension().unwrap_or_default().to_str().unwrap();
        let hashed_fname = fname + "." + extension;
        Self {
            path_hash: digest(path.to_path_buf().into_os_string().into_string().unwrap()),
            filename: hashed_fname,
            v_language: String::from(lang),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct RunInfo {
    aliases: Vec<String>,
    repos: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ErrorInfo {
    errors: Vec<String>
}