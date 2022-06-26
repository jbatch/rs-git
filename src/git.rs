use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io::Read,
    io::Write,
    path::Path,
};

use flate2::write::ZlibEncoder;
use flate2::{read::ZlibDecoder, Compression};
use sha1::{Digest, Sha1};

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

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

impl Object {
    pub fn read_from_sha1(object_sha: &str) -> Result<Object> {
        let (prefix, suffix) = (&object_sha[..2], &object_sha[2..]);
        let bytes = get_object_file_as_byte_vec(prefix, suffix)?;
        let contents = zlib_decompress(bytes)?;
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
        };
        let data_bin = zlib_compress(data)?;
        file.write(&data_bin)?;
        Ok(())
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

fn zlib_decompress(bytes: Vec<u8>) -> Result<String> {
    let mut z = ZlibDecoder::new(&bytes[..]);
    let mut s = String::new();
    z.read_to_string(&mut s)?;
    Ok(s)
}

fn zlib_compress(s: String) -> Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write(s.as_bytes())?;
    let compressed = e.finish()?;
    Ok(compressed)
}
