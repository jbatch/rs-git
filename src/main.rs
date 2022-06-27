// use clap::{Parser, Subcommand};
use std::fs::{self};

mod git;
pub use git::*;
mod args;
pub use args::*;

fn main() -> crate::Result<()> {
    let args = Args::parse()?;

    let result = match args.command {
        Command::Init {} => init(),
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
        Command::HashObject { write_object, file } => {
            hash_object(Command::HashObject { write_object, file })
        }
    };
    if let Err(why) = result {
        println!("fatal: {}", &why);
        std::process::exit(9);
    }
    std::process::exit(0);
}

fn init() -> Result<()> {
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/master\n").unwrap();
    println!("Initialized git directory");
    Ok(())
}

fn cat_file(command: Command) -> Result<()> {
    if let Command::CatFile {
        print_type,
        print_size,
        pretty_print,
        object,
    } = command
    {
        let obj = Object::read_from_sha1(&object)?;
        match obj {
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
            Object::Tree { len, entries } => {
                if print_type {
                    println!("tree");
                }
                if print_size {
                    println!("{}", len);
                }
                if pretty_print {
                    for entry in entries {
                        println!("{} {} {}\t{}", entry.mode, "blob", entry.sha1, entry.name);
                    }
                }
            }
        }

        Ok(())
    } else {
        panic!("Unreachable");
    }
}

fn hash_object(command: Command) -> Result<()> {
    if let Command::HashObject { write_object, file } = command {
        let object = Object::read_from_file(&file)?;
        let sha1_hash = object.get_sha1()?;
        println!("{}", sha1_hash);
        if write_object {
            object.write_to_database()?;
        }
        Ok(())
    } else {
        panic!("Unreachable");
    }
}
