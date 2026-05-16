use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: String,
    pub icon: Option<String>,
    pub source: PathBuf,
}

impl DesktopEntry {
    pub fn subtitle(&self) -> &str {
        self.comment
            .as_deref()
            .or(self.generic_name.as_deref())
            .unwrap_or(&self.exec)
    }

    pub fn rank(&self, query: &str) -> Option<i32> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Some(100);
        }

        let name = self.name.to_lowercase();
        if name == query {
            return Some(0);
        }
        if name.starts_with(&query) {
            return Some(10);
        }
        if name.contains(&query) {
            return Some(20);
        }

        if self
            .generic_name
            .as_deref()
            .map(|v| v.to_lowercase().contains(&query))
            .unwrap_or(false)
        {
            return Some(40);
        }

        if self
            .comment
            .as_deref()
            .map(|v| v.to_lowercase().contains(&query))
            .unwrap_or(false)
        {
            return Some(60);
        }

        None
    }
}
