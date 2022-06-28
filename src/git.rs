use std::str;
use std::{
    error::Error,
    fmt,
    fs::{self, DirEntry, File},
    io::Read,
    io::Write,
    num::ParseIntError,
    os::unix::prelude::PermissionsExt,
    path::Path,
    vec::IntoIter,
};

use flate2::write::ZlibEncoder;
use flate2::{read::ZlibDecoder, Compression};
use sha1::{Digest, Sha1};

#[derive(Debug)]
pub enum Object {
    Blob { len: i32, content: String },
    Tree { len: i32, entries: Vec<Entry> },
}

#[derive(Debug)]
pub struct Entry {
    pub mode: u32,
    pub type_: String,
    pub name: String,
    pub sha1: String,
}

impl Entry {
    pub fn new(bytes: &mut IntoIter<u8>) -> Result<Entry> {
        let mode = String::from_utf8(take_until(bytes, b' '))?.parse::<u32>()?;
        let name = String::from_utf8(take_until(bytes, b'\0'))?;
        let type_ = if mode.to_string().chars().nth(0).unwrap() == '1' {
            "blob".to_string()
        } else {
            "tree".to_string()
        };
        let sha1 = bytes
            .by_ref()
            .take(20)
            .map(|b| format!("{:02x}", b))
            .collect();

        Ok(Entry {
            mode,
            type_,
            name,
            sha1,
        })
    }

    pub fn from_dir_entry(dir_entry: DirEntry) -> Result<Entry> {
        let metadata = dir_entry.metadata()?;
        let is_dir = metadata.is_dir();
        let path = dir_entry.path();
        let object = if is_dir {
            Object::read_from_dir(&path)?
        } else {
            Object::from_path(&path)?
        };
        let sha1 = object.get_sha1()?;
        let type_ = match object {
            Object::Blob { len: _, content: _ } => "blob".to_string(),
            Object::Tree { len: _, entries: _ } => "tree".to_string(),
        };
        let name = dir_entry
            .file_name()
            .into_string()
            .map_err(|os| GitError::CorruptFile())?;
        let mode = Self::get_mode(&dir_entry)?;
        Ok(Entry {
            mode,
            type_,
            name,
            sha1,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        //// [mode] [file name]\0[object ID]
        vec![
            format!("{:o}", self.mode).as_bytes().to_vec(),
            " ".as_bytes().to_vec(),
            self.name.as_bytes().to_vec(),
            "\0".as_bytes().to_vec(),
            decode_hex(&self.sha1).unwrap(),
        ]
        .concat()
    }

    pub fn len(&self) -> i32 {
        self.to_bytes().len() as i32
    }

    fn get_mode(dir_entry: &DirEntry) -> Result<u32> {
        // From https://stackoverflow.com/questions/737673/how-to-read-the-mode-field-of-git-ls-trees-output
        Ok(dir_entry.metadata()?.permissions().mode())
    }
}

#[derive(Debug, Clone)]
pub enum GitError {
    InvalidArgs(String),
    CorruptFile(),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitError::CorruptFile() => write!(f, "Could not read corrupted file"),
            GitError::InvalidArgs(why) => write!(f, "{}", &why),
        }
    }
}

impl Error for GitError {}

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

impl Object {
    pub fn read_from_sha1(object_sha: &str) -> Result<Object> {
        let (prefix, suffix) = (&object_sha[..2], &object_sha[2..]);
        let bytes = get_object_file_as_byte_vec(prefix, suffix)?;
        let mut rest = zlib_decompress(bytes)?.into_iter();
        let obj_type = take_until(&mut rest, b' ');
        let obj_type = String::from_utf8(obj_type)?;
        let len = String::from_utf8(take_until(&mut rest, b'\0'))?
            .parse::<i32>()
            .unwrap();

        // let (obj_type, rest) = contents.split_once(' ').ok_or(GitError::CorruptFile())?;
        match obj_type.as_str() {
            "blob" => {
                // Blob format: {type} {len}\0{content}
                // Split bytes at next NUL byte and extract as the length.

                let content = String::from_utf8(rest.collect())?;
                Ok(Self::Blob { len, content })
            }
            "tree" => {
                // Tree format: {type} {len}\0[{mode} {file/dir name}\0{SHA1 hash}]*
                // where the {SHA1 hash} is binary.
                let mut entries = Vec::new();
                while rest.len() > 0 {
                    let entry = Entry::new(&mut rest)?;
                    entries.push(entry);
                }
                Ok(Self::Tree { len, entries })
            }
            _ => Err(Box::new(GitError::CorruptFile())),
        }
    }

    pub fn from_path(path: &Path) -> Result<Object> {
        let content = fs::read_to_string(path)?;
        let len = content.len() as i32;

        Ok(Self::Blob { len, content })
    }

    pub fn read_from_dir(dir: &Path) -> Result<Object> {
        let dir = fs::read_dir(dir)?;
        println!("read_from_dir {:?}", &dir);
        let mut len = 0;
        let mut entries: Vec<Entry> = Vec::new();

        for entry in dir {
            let entry = entry?;
            // Filter out ignored files
            let ignored_names = ["target".to_string(), ".git".to_string()];
            if ignored_names
                .iter()
                .any(|v| v.eq(&entry.file_name().into_string().unwrap()))
            {
                continue;
            }
            println!("Creating entry from {:?}", entry);
            let e = Entry::from_dir_entry(entry)?;
            println!("Created entry {:?}", e);
            len += e.len();
            entries.push(e);
        }
        Ok(Self::Tree { len, entries })
    }

    pub fn get_sha1(&self) -> Result<String> {
        match self {
            Object::Blob { len, content } => {
                let s = format!("{} {}\0{}", "blob", len, content);
                let bytes = Sha1::digest(s.as_bytes());
                Ok(format!("{:x}", bytes))
            }
            Object::Tree { len, entries } => {
                // Format: {type} {len}\0[{mode} {file/dir name}\0{SHA1 hash}]*
                // where the {SHA1 hash} is binary.
                let mut bytes = vec![
                    "tree".as_bytes().to_vec(),
                    " ".as_bytes().to_vec(),
                    len.to_string().as_bytes().to_vec(),
                    "\0".as_bytes().to_vec(),
                ];
                for e in entries {
                    bytes.push(e.to_bytes())
                }
                let hash = Sha1::digest(&bytes.concat());
                println!("creating hash {:?} => {:?}", bytes.concat(), hash);
                Ok(format!("{:x}", hash))
            }
        }
    }

    pub fn write_to_database(&self) -> Result<()> {
        let sha1 = self.get_sha1()?;
        let (prefix, suffix) = (&sha1[..2], &sha1[2..]);
        let path = Path::new(".git").join("objects").join(prefix).join(suffix);
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut file = File::create(path)?;
        let data = match self {
            Object::Blob { len, content } => format!("blob {}\0{}", len, content),
            Object::Tree { len: _, entries: _ } => todo!(),
        };
        let data_bin = zlib_compress(data)?;
        file.write(&data_bin)?;
        Ok(())
    }
}

/// Takes in an itterable of bytes and returns a Vec of bytes the the left of the target or the whole Iterable if target not found.
fn take_until<'a>(bytes: &mut IntoIter<u8>, target: u8) -> Vec<u8> {
    let type_buf: Vec<u8> = bytes.by_ref().take_while(|b| *b != target).collect();
    type_buf
}

fn get_object_file_as_byte_vec(prefix: &str, suffix: &str) -> Result<Vec<u8>> {
    let path = Path::new(".git").join("objects").join(prefix).join(suffix);
    let mut f = File::open(&path)?;
    let metadata = fs::metadata(&path).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");
    Ok(buffer)
}

fn zlib_decompress(bytes: Vec<u8>) -> Result<Vec<u8>> {
    let mut z = ZlibDecoder::new(&bytes[..]);
    let mut buf: Vec<u8> = Vec::new();
    z.read_to_end(&mut buf)?;
    Ok(buf)
}

fn zlib_compress(s: String) -> Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write(s.as_bytes())?;
    let compressed = e.finish()?;
    Ok(compressed)
}

//Helpers from https://stackoverflow.com/questions/52987181/how-can-i-convert-a-hex-string-to-a-u8-slice

pub fn decode_hex(s: &str) -> std::result::Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        fmt::write(&mut s, format_args!("{:02x}", b)).unwrap();
    }
    s
}
