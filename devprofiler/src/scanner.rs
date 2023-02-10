use std::path::PathBuf;
use walkdir::WalkDir;
use crate::observer::RuntimeInfo;
use crate::writer::OutputWriter;
use std::io;
use std::io::Write;

pub struct RepoScanner {
    scanpath: PathBuf
}

impl RepoScanner {
    pub fn new(scanpath: PathBuf) -> Self{
        Self { scanpath }
    }

    pub fn scan(&self, einfo: &mut RuntimeInfo, writer: &mut OutputWriter, dockermode: bool) -> Vec<String>{
        let walker = WalkDir::new(self.scanpath.as_path()).into_iter();
        let mut repo_paths = Vec::<String>::new();
        let mut scan_err = false;
        let mut count = 0;
        for entry in walker.filter_map(|elem| {
            if elem.is_err() {
                let err_str = elem.err().expect("Checked, is err")
                    .to_string();
                einfo.record_err(&err_str);
                match writer.write_io_err(&err_str) {
                    Ok(_) => {},
                    Err(error) => { 
                        scan_err = true;
                        einfo.record_err(
                            error.to_string().as_str().as_ref());
                    }
                }
                None
            }
            else{
                elem.ok()
            }
        }) 
        {
            count += 1;
            if !dockermode {
                Self::print_progress(count);
            }
            let path = entry.path();
            if path.ends_with(".git") {
                repo_paths.push(
                    path.parent()
                    .expect("None only when path = /")
                    .to_str().expect("None only when path = /")
                    .to_string());
            }
        }
        if scan_err {
            eprintln!("Some directories were inaccessible. I/O errors are detailed in io_errors.txt");
        }
        repo_paths
    }

    fn print_progress(count: i64) {
        let mut v = vec!["Scanning directories "];
            if count % 4 == 0 {v.push("/");}
            if count % 4 == 1 {v.push("-");}
            if count % 4 == 2 {v.push("\\");}
            if count % 4 == 3 {v.push("-");}
            v.push(" \r");
            print!("{}", v.concat());
		    let _res = io::stdout().flush();
    }
}