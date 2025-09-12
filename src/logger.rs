use colored::Colorize;

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum Level {
    /// Something really went wrong and the user will experience problems.
    Error,
    /// Generally silent, except something occurs that may lead to failure or
    /// unexpected behavior
    Warning,
    /// Normal informative output to follow the control flow
    Info,
    /// Fine-grained output to understand the program behavior down to
    /// implementation details
    Debug,
}

#[derive(Clone, Copy)]
pub struct Logger {
    verbosity: Level,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            verbosity: Level::Debug, // TODO default should be Info
        }
    }

    pub fn debug<S: AsRef<str>>(&self, msg: S) {
        if self.verbosity >= Level::Debug {
            println!("{}", msg.as_ref().cyan());
        }
    }

    pub fn info<S: AsRef<str>>(&self, msg: S) {
        if self.verbosity >= Level::Info {
            println!("{}", msg.as_ref());
        }
    }

    pub fn warn<S: AsRef<str>>(&self, msg: S) {
        if self.verbosity >= Level::Warning {
            eprintln!("{}", msg.as_ref().yellow());
        }
    }

    pub fn error<S: AsRef<str>>(&self, msg: S) {
        if self.verbosity >= Level::Error {
            eprintln!("{}", msg.as_ref().red());
        }
    }
}
