use std::path::PathBuf;
use walkdir::WalkDir;
use crate::observer::RuntimeInfo;

pub struct RepoScanner {
    scanpath: PathBuf
}

impl RepoScanner {
    pub fn new(scanpath: PathBuf) -> Self{
        Self { scanpath }
    }

    pub fn scan(&self, einfo: &mut RuntimeInfo) -> Vec<String>{
        let walker = WalkDir::new(self.scanpath.as_path()).into_iter();
        let mut repo_paths = Vec::<String>::new();
        for entry in walker.filter_map(|elem| {
            if elem.is_err() {
                einfo.push(elem.err().expect("Checked, is err")
                            .to_string().as_str().as_ref());
                None
            }
            else{
                elem.ok()
            }
        }) 
        {
            let path = entry.path();
            if path.ends_with(".git") {
                repo_paths.push(
                    path.parent()
                    .expect("None only when path = /")
                    .to_str().expect("None only when path = /")
                    .to_string());
            }
        }
        repo_paths
    }
}