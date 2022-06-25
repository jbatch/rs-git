// use clap::{Parser, Subcommand};
use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::Read,
    path::Path,
};

use flate2::read::ZlibDecoder;

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
    fn parse() -> Result<Args> {
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

#[derive(Debug)]
pub enum Object {
    Blob { len: i32, content: String },
}

#[derive(Debug, Clone)]
pub enum GitError {
    InvalidArgs(),
    CorruptFile(),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitError::CorruptFile() => write!(f, "Could not read corrupted file"),
            GitError::InvalidArgs() => write!(f, "Invalid command line args"),
        }
    }
}

impl Error for GitError {}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

impl Object {
    pub fn read_from_sha1(object_sha: &str) -> Result<Object> {
        let (prefix, suffix) = (&object_sha[..2], &object_sha[2..]);
        let bytes = get_object_file_as_byte_vec(prefix, suffix)?;
        let contents = decode_reader(bytes)?;
        let (obj_type, rest) = contents.split_once(' ').ok_or(GitError::CorruptFile())?;
        match obj_type {
            "blob" => {
                let (object_len, rest) = rest
                    .split_once('\0')
                    .map(|(s1, s2)| (s1.parse::<i32>().unwrap(), s2)) // TODO get rid of unwrap
                    .ok_or(GitError::CorruptFile())?;
                return Ok(Self::Blob {
                    len: object_len,
                    content: rest.to_string(),
                });
            }
            _ => Err(Box::new(GitError::CorruptFile())),
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse()?;

    match args.command {
        Command::Init {} => {
            init();
        }
        Command::CatFile {
            pretty_print,
            object,
            print_type,
            print_size,
        } => cat_file(Command::CatFile {
            pretty_print,
            object,
            print_type,
            print_size,
        }),
    }
    Ok(())
}

fn init() {
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
    println!("Initialized git directory")
}

fn cat_file(command: Command) {
    if let Command::CatFile {
        print_type,
        print_size,
        pretty_print,
        object,
    } = command
    {
        match Object::read_from_sha1(&object) {
            Ok(obj) => match obj {
                Object::Blob { len, content } => {
                    if print_type {
                        println!("blob");
                    }
                    if print_size {
                        println!("{}", len);
                    }
                    if pretty_print {
                        print!("{}", content);
                    }
                }
            },
            Err(why) => println!("Err: {}", why),
        }
    }
}

fn get_object_file_as_byte_vec(prefix: &str, suffix: &str) -> Result<Vec<u8>> {
    let path = Path::new(".git").join("objects").join(prefix).join(suffix);
    let mut f = File::open(&path)?;
    let metadata = fs::metadata(&path).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");
    Ok(buffer)
}

fn decode_reader(bytes: Vec<u8>) -> Result<String> {
    let mut z = ZlibDecoder::new(&bytes[..]);
    let mut s = String::new();
    z.read_to_string(&mut s)?;
    Ok(s)
}
