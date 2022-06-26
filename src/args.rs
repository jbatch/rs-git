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
            None => Err(GitError::InvalidArgs()),
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
                    None => Err(GitError::InvalidArgs()),
                }
            }
            _ => Err(GitError::InvalidArgs()),
        }?;

        Ok(Args { command })
    }
}
