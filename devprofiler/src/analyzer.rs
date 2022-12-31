use git2::{ Repository, Diff, Commit };
use detect_lang;
use serde::Serialize;
use sha256::digest;
use std::path::PathBuf;
use std::path::Path;
use std::error::Error;
use std::collections::HashSet;
use crate::writer::OutputWriter;
use crate::observer::RuntimeInfo;

pub struct RepoAnalyzer {
    repo: Repository,
    path: PathBuf, 
}

impl RepoAnalyzer {
    pub fn new(path_str: String) -> Result<RepoAnalyzer, Box<dyn Error>> {
        let path = Path::new(&path_str);
        let repo = Repository::discover(&path)?;
        Ok(Self {
            path: path.to_owned(),
            repo: repo,
        })
    }

    pub fn analyze(&self, writer: &mut OutputWriter, einfo: &mut RuntimeInfo) 
        -> Result<HashSet::<String>, Box<dyn Error>>{
        let mut aliases = HashSet::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        for rev in revwalk {
            match rev {
                Ok(objid) => {
                    let commit_res = self.repo.find_commit(objid);
                    match commit_res {
                        Ok(commit) => {
                            aliases.insert(commit.author().email().unwrap_or_default().to_string());
                            let cinfo = self.extract_commit_obj(&commit);
                            let serialized = serde_json::to_string(&cinfo).unwrap_or_default();
                            match writer.writeln(serialized.as_str().as_ref()) {
                                Ok(_) => {},
                                Err(writer_err) => {
                                    einfo.push(writer_err.to_string().as_str().as_ref());
                                }
                            }
                        },
                        Err(commit_err) => {
                            einfo.push(commit_err.to_string().as_str().as_ref());
                        }
                    }
                },
                Err(rev_err) => {
                    einfo.push(rev_err.to_string().as_str().as_ref());
                }
            }
        }
        Ok(aliases)
    }

    fn extract_reponame(&self) -> &str{
        self.path.as_path()
            .strip_prefix(self.path.as_path().parent().expect("None only if path = /"))
            .expect("Err only if non-parent/prefix argument")
            .as_os_str().to_str().expect("None only if path is empty").as_ref()
    }

    fn extract_commit_obj(&self, commit: &Commit) -> CommitInfo {
        let diff = self.extract_diff(commit);
        let cinfo = CommitInfo::new(commit, &diff, self.extract_reponame());
        cinfo
    }

    fn extract_diff(&self, commit: &Commit) -> Option<Diff> {
        let mut diff: Option<Diff> = None;
        let commit_tree = commit.tree();
        // TODO - diff should be taken form all parents and intersected
        let parent = commit.parent(0);
        if parent.is_ok() && commit_tree.is_ok(){
            let parent_tree = parent.expect("Checked, is ok").tree();
            let diff_result = self.repo
                .diff_tree_to_tree(
                    Some(&(commit_tree.expect("Checked, is ok"))), 
                    Some(&(parent_tree.expect("Checked, is ok"))),
                    None
                );
            if diff_result.is_ok() {
                diff = Some(diff_result.expect("Checked, is ok"));
            }
        }
        diff
    }
}

#[derive(Clone, Debug, Serialize, Default)]
struct DiffInfo {
    insertions: usize,
    deletions: usize,
    files_changed: usize,
    file_info: Vec<DiffFileInfo>,
}

#[derive(Clone, Debug, Serialize, Default)]
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
    fn new(commit: &Commit, diff: &Option<Diff>, reponame: &str) -> Self {
        let tsecs = commit.time().seconds();
        let toffset :i64 = commit.time().offset_minutes().into();
        let mut cparents :Vec<String>  = Vec::new();
        for c in commit.parents() {
            cparents.push(digest(c.id().to_string()));
        }
        Self {
            commit_id: digest(commit.id().to_string()),
            repo_name: reponame.to_string(),
            author_name: digest(commit.author().name().unwrap_or_default().to_string()),
            author_email: digest(commit.author().email().unwrap_or_default().to_string()),
            ts_secs: tsecs,
            ts_offset_mins: toffset,
            parents: cparents,
            diff_info: Self::get_diffs(diff).unwrap_or_default(),
        }
    }
    
    fn get_diffs(diff: &Option<Diff>) -> Option<DiffInfo>{
        if diff.is_none() {
            return None;
        }
        let diff_obj = diff.as_ref().expect("Checked, is not none");
        let mut diffvec: Vec<DiffFileInfo> = Vec::new();
        for delta in diff_obj.deltas() {
            let fpath = delta.new_file().path();
            if fpath.is_some() {
                let filepath = fpath.expect("fpath is some, conditional check on block");
                let lang = match detect_lang::from_path(filepath) {
                    Some(langid) => langid.id(),
                    None => "None",
                };
                diffvec.push(DiffFileInfo::new(&filepath, lang));
            }
        }
        let stats = diff_obj.stats();
        match stats.is_ok() {
            true => {
                let stats_obj = stats.expect("stats is ok, conditional check on block");
                Some(DiffInfo {
                    insertions: stats_obj.insertions(),
                    deletions: stats_obj.deletions(),
                    files_changed: stats_obj.files_changed(),
                    file_info: diffvec,
                })
            }
            false => None
        }
    }
}

impl DiffFileInfo {
    fn new(path: &Path, lang: &str) -> Self {
        let stemname = digest(
            path.file_stem()
            .expect("Not none as filename must exist in git history")
            .to_str().unwrap_or_default().to_string());
        let extension = path.extension().unwrap_or_default().to_str();
        let hashed_fname = match extension.is_none() {
            true => stemname,
            false => stemname + "." + extension.expect("extension is some, conditional check on block"),
        };
        Self {
            path_hash: digest(path.to_path_buf().into_os_string().into_string().unwrap_or_default()),
            filename: hashed_fname,
            v_language: lang.to_string().to_owned(),
        }
    }
}