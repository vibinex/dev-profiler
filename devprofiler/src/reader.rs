use inquire::{
    error::InquireResult,
    MultiSelect,
    Text,
};

pub struct UserInput {}

impl UserInput {
    pub fn scan_path() -> InquireResult<String>{
        Text::new("Enter path containing one or more git repo(s)").prompt()
    }

    pub fn repo_selection(options: Vec::<String>) -> InquireResult<Vec::<String>>{
        MultiSelect::new(
            "Select relevant repo(s)", options)
            .prompt()
    }

    pub fn alias_selector(options: Vec::<String>) -> InquireResult<Vec::<String>>{
        MultiSelect::new(
            "Select your email aliases", options)
            .prompt()
    }
}