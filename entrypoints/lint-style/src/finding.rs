use std::{fmt, path::PathBuf};

pub struct Finding {
    path: PathBuf,
    line: usize,
    level: Level,
    message: String,
}

impl Finding {
    pub fn error(path: PathBuf, line: usize, message: impl Into<String>) -> Self {
        Self::new(path, line, Level::Error, message)
    }

    pub fn debt(path: PathBuf, line: usize, message: impl Into<String>) -> Self {
        Self::new(path, line, Level::Debt, message)
    }

    fn new(path: PathBuf, line: usize, level: Level, message: impl Into<String>) -> Self {
        Self {
            path,
            line,
            level,
            message: message.into(),
        }
    }

    #[cfg(test)]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: [{}] {}", self.path.display(), self.line, self.level.as_str(), self.message)
    }
}

enum Level {
    Error,
    Debt,
}

impl Level {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Debt => "debt",
        }
    }
}
