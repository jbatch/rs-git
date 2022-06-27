use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::Read,
    io::Write,
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
    pub mode: i32,
    pub type_: String,
    pub name: String,
    pub sha1: String,
}

impl Entry {
    pub fn new(bytes: &mut IntoIter<u8>) -> Result<Entry> {
        let mode = String::from_utf8(take_until(bytes, b' '))?.parse::<i32>()?;
        let type_ = if mode.to_string().chars().nth(0).unwrap() == '1' {
            "blob".to_string()
        } else {
            "tree".to_string()
        };
        let name = String::from_utf8(take_until(bytes, b'\0'))?;
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

    pub fn read_from_file(file: &String) -> Result<Object> {
        let path = Path::new(&file);
        let content = fs::read_to_string(path)?;
        let len = content.len() as i32;

        Ok(Self::Blob { len, content })
    }

    pub fn get_sha1(&self) -> Result<String> {
        match self {
            Object::Blob { len, content } => {
                let s = format!("{} {}\0{}", "blob", len, content);
                let bytes = Sha1::digest(s.as_bytes());
                Ok(format!("{:x}", bytes))
            }
            Object::Tree { len: _, entries: _ } => todo!(),
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
