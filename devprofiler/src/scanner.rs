use std::path::PathBuf;
use walkdir::{WalkDir, Error};

pub struct RepoScanner {
    scanpath: PathBuf
}

impl RepoScanner {
    pub fn new(scanpath: PathBuf) -> Self{
        Self { scanpath }
    }

    pub fn scan(&self) -> (Vec<PathBuf>, Vec<Error>){
        let walker = WalkDir::new(self.scanpath.as_path()).into_iter();
        let mut repo_paths = Vec::<PathBuf>::new();
        let mut repo_errs = Vec::<Error>::new();
        for entry in walker.filter_map(|elem| {
            if elem.is_err() {
                let err = elem.err().expect("Cannot be none, checked for error before entring block");
                eprintln!("Unable to read directory/file: {:?}", err);
                repo_errs.push(err);
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
                    .expect("Checking last component, should always have a parent")
                    .to_owned());
            }
        }
        (repo_paths, repo_errs)
    }
}