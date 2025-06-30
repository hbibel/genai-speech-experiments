use colored::Colorize;

pub trait Logger: Send {
    fn debug(&self, msg: &str);

    fn info(&self, msg: &str);

    fn warn(&self, msg: &str);

    fn error(&self, msg: &str);
}

#[derive(PartialEq, PartialOrd)]
pub enum Level {
    Info,
    Debug,
}

pub struct ConsoleLogger {
    verbosity: Level,
}

impl ConsoleLogger {
    pub fn new() -> Self {
        Self {
            verbosity: Level::Debug, // TODO default should be Info
        }
    }
}

impl Logger for ConsoleLogger {
    fn debug(&self, msg: &str) {
        if self.verbosity >= Level::Debug {
            println!("{}", msg.cyan());
        }
    }

    fn info(&self, msg: &str) {
        println!("{msg}");
    }

    fn warn(&self, msg: &str) {
        eprintln!("{}", msg.yellow());
    }

    fn error(&self, msg: &str) {
        eprintln!("{}", msg.red());
    }
}
