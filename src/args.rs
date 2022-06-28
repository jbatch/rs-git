use super::{GitError, Result};

#[derive(Debug, Clone)]
// #[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Init {},
    // #[clap(name = "cat-file")]
    CatFile {
        /// show object type
        // #[clap(name = "type", short = 't', value_parser)]
        print_type: bool,
        // #[clap(name = "size", short = 's', value_parser)]
        /// show object size
        print_size: bool,
        // #[clap(short = 'p', value_parser)]
        /// pretty print the object's contents
        pretty_print: bool,
        object: String,
    },
    HashObject {
        /// write the object into the object database
        // #[clap( short = 'w', value_parser)]
        write_object: bool,
        /// file to hash
        file: String,
    },
    LsTree {
        /// Whether to only print the file/dir names.
        name_only: bool,
        /// Hash of the tree to print
        object: String,
    },
    WriteTree {},
}

// #[derive(Parser, Debug)]
#[derive(Debug)]
// #[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Command to run
    // #[clap(subcommand)]
    pub command: Command,
}

impl Args {
    pub fn parse() -> Result<Args> {
        let args = std::env::args();
        // Skip executable name
        let mut args = args.skip(1).peekable();

        let command = match args.next() {
            Some(command) => Ok(command),
            None => Err(GitError::InvalidArgs("missing command".to_string())),
        }?;

        let command = match command.as_str() {
            "init" => Ok(Command::Init {}),
            "cat-file" => {
                let print_size = false;
                let print_type = false;
                let mut pretty_print = false;
                let mut object: Option<String> = None;
                while let Some(arg) = args.peek() {
                    if arg.starts_with("-") {
                        if arg == "-p" {
                            pretty_print = true;
                        }
                        args.next().unwrap();
                    } else {
                        // treat as positional arg <object>
                        object = Some(args.next().unwrap());
                    }
                }
                match object {
                    Some(object) => Ok(Command::CatFile {
                        print_type,
                        print_size,
                        pretty_print,
                        object,
                    }),
                    None => Err(GitError::InvalidArgs(
                        "missing positional argument <object>".to_string(),
                    )),
                }
            }
            "hash-object" => {
                let mut write_object = false;
                let mut file: Option<String> = None;
                while let Some(arg) = args.peek() {
                    if arg.starts_with("-") {
                        if arg == "-w" {
                            write_object = true;
                        }
                        args.next().unwrap();
                    } else {
                        // treat as positional arg <file>
                        file = Some(args.next().unwrap());
                    }
                }
                match file {
                    Some(file) => Ok(Command::HashObject { write_object, file }),
                    None => Err(GitError::InvalidArgs(
                        "missing positional argument <file>".to_string(),
                    )),
                }
            }
            "ls-tree" => {
                let mut name_only = false;
                let mut object = None;
                while let Some(arg) = args.peek() {
                    if arg.starts_with("-") {
                        if arg == "--name-only" {
                            name_only = true;
                        }
                        args.next().unwrap();
                    } else {
                        // treat as positional arg <object>
                        object = Some(args.next().unwrap());
                    }
                }
                match object {
                    Some(object) => Ok(Command::LsTree { name_only, object }),
                    None => Err(GitError::InvalidArgs(
                        "missing potisional argument <tree>".to_string(),
                    )),
                }
            }
            "write-tree" => Ok(Command::WriteTree {}),
            _ => Err(GitError::InvalidArgs(format!(
                "invalid command: {}",
                command
            ))),
        }?;

        Ok(Args { command })
    }
}
