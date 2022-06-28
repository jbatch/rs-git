// use clap::{Parser, Subcommand};
use std::{
    env,
    fs::{self},
    path::Path,
};

mod git;
pub use git::*;
mod args;
pub use args::*;

fn main() -> crate::Result<()> {
    let args = Args::parse();
    if let Err(why) = args {
        println!("fatal: {}", &why);
        std::process::exit(9);
    }
    let args = args.unwrap();

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
        Command::LsTree { name_only, object } => ls_tree(Command::LsTree { name_only, object }),
        Command::WriteTree {} => write_tree(Command::WriteTree {}),
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
                        println!(
                            "{:06} {} {}\t{}",
                            entry.mode, entry.type_, entry.sha1, entry.name
                        );
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
        let object = Object::from_path(&Path::new(&file))?;
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

fn ls_tree(command: Command) -> Result<()> {
    if let Command::LsTree { name_only, object } = command {
        let obj = Object::read_from_sha1(&object)?;
        if let Object::Tree { len: _, entries } = obj {
            for entry in entries {
                if name_only {
                    println!("{}", entry.name);
                } else {
                    println!(
                        "{:06} {} {}\t{}",
                        entry.mode, "blob", entry.sha1, entry.name
                    );
                }
            }
        } else {
            return Err(Box::new(GitError::InvalidArgs(
                "not a tree object".to_string(),
            )));
        }
        Ok(())
    } else {
        panic!("Unreachable");
    }
}

fn write_tree(command: Command) -> Result<()> {
    if let Command::WriteTree {} = command {
        let dir = Object::read_from_dir(&env::current_dir()?)?;
        dir.write_to_database()?;
        println!("{}", dir.get_sha1()?);
        Ok(())
    } else {
        panic!("unreachable");
    }
}
