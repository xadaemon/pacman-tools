use crate::ProgramError;
use crate::Result;
use flate2::read::MultiGzDecoder;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};

#[cfg(feature = "serde-types")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    name: String,
    metadata: HashMap<String, Vec<String>>,
}

#[cfg(not(feature = "serde-types"))]
#[derive(Debug)]
pub struct Package {
    name: String,
    metadata: HashMap<String, Vec<String>>,
}

#[cfg(feature = "serde-types")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    file: PathBuf,
    signed: bool,
    packages: HashMap<String, Package>,
}

#[cfg(not(feature = "serde-types"))]
#[derive(Debug)]
pub struct Database {
    file: PathBuf,
    signed: bool,
    packages: HashMap<String, Package>,
}

impl Database {
    pub fn open(pth: &Path) -> Result<Self> {
        if !pth.try_exists()? {
            return Err(ProgramError::ENOF(pth.to_path_buf()));
        }

        let mut rdr = File::open(&pth)?;

        let mut dec_data = Vec::with_capacity(0xa000);
        Self::decompress(&mut rdr, &mut dec_data)?;

        let mut tar_handler = tar::Archive::new(BufReader::new(Cursor::new(dec_data)));
        let mut packages = HashMap::new();

        for entry in tar_handler.entries()? {
            let mut entry = entry?;
            if !&entry.path()?.ends_with("desc") {
                continue;
            }

            let mut entry_data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut entry_data)?;

            let desc = String::from_utf8(entry_data).unwrap();
            let package = Self::parse_entry_desc(desc)?;

            packages.insert(package.name.clone(), package);
        }

        Ok(Database {
            file: pth.to_path_buf(),
            signed: false,
            packages,
        })
    }

    fn parse_entry_desc(data: String) -> Result<Package> {
        let mut metadata = HashMap::new();
        let lines = data.lines();
        let mut key = "".to_owned();
        let mut val = Vec::new();
        let mut inside_block = false;
        for line in lines {
            if line.starts_with("%") {
                if inside_block {
                    metadata.insert(key.clone(), val.clone());
                    val.clear();
                }
                inside_block = true;
                let striped = line.replace("%", "");
                key = striped.to_lowercase();
            } else if !line.is_empty() {
                val.push(line.to_owned());
            }
        }

        Ok(Package {
            name: metadata.get("name").unwrap()[0].clone(),
            metadata,
        })
    }

    fn decompress(rdr: &mut File, buf: &mut Vec<u8>) -> Result<usize> {
        let buf_rdr = BufReader::new(&mut *rdr);
        let mut zstd_dec = zstd::Decoder::new(buf_rdr)?;

        match zstd_dec.read_to_end(buf) {
            Ok(r) => Ok(r),
            Err(_) => {
                buf.clear();
                rdr.rewind()?;
                Self::try_gzip(rdr, buf)
            }
        }
    }

    fn try_gzip(rdr: &mut File, buf: &mut Vec<u8>) -> Result<usize> {
        let mut gzip_dec = MultiGzDecoder::new(rdr);
        gzip_dec.read_to_end(buf)?;
        Ok(0)
    }

    pub fn file(&self) -> &Path {
        self.file.as_path()
    }

    pub fn packages(&self) -> &HashMap<String, Package> {
        &self.packages
    }

    pub fn lookup(&self, pkg: &str) -> Option<&Package> {
        self.packages.get(pkg)
    }
}

impl Package {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn metadata(&self) -> &HashMap<String, Vec<String>> {
        &self.metadata
    }
}
