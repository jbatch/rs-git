// use clap::{Parser, Subcommand};
use std::fs::{self};

mod git;
pub use git::*;
mod args;
pub use args::*;

fn main() -> crate::Result<()> {
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
