use clap::{Parser, Subcommand};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read};
use std::{collections::HashMap, io, path::PathBuf};
use clap::builder::Str;
use thiserror::Error;
use zstd::bulk::Decompressor;

type Result<T> = std::result::Result<T, ProgramError>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// force a db file to be used
    db: Option<PathBuf>,
    /// Print debug info
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: Option<u8>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// print information about a package
    PkgInfo { pkg_name: String },
}

#[derive(Debug)]
struct ProgState {
    db: Option<PathBuf>,
    debug_lvl: u8,
}

#[derive(Error, Debug)]
pub enum ProgramError {
    #[error("An io Error ocurred {0}")]
    EIO(#[from] io::Error),
    #[error("File {0} not found")]
    ENOF(PathBuf),
}

#[derive(Debug)]
struct Package {
    name: String,
    metadata: HashMap<String, Vec<String>>
}

#[derive(Debug)]
struct Database {
    file: PathBuf,
    signed: bool,
    packages: Box<HashMap<String, Package>>,
}

impl Database {
    fn open(pth: PathBuf) -> Result<Self> {
        if !pth.try_exists()? {
            return Err(ProgramError::ENOF(pth));
        }

        let rdr = File::open(&pth)?;
        let buf = BufReader::new(rdr);

        let mut dec = zstd::Decoder::new(buf)?;
        let mut dec_data = Vec::with_capacity(0xa000);
        let read = dec.read_to_end(&mut dec_data)?;

        let mut tar_handler = tar::Archive::new(BufReader::new(Cursor::new(dec_data)));
        let mut packages = HashMap::new();

        for entry in tar_handler.entries()? {
            let mut entry = entry?;
            if !&entry.path()?.ends_with("desc") {
                continue;
            }

            let path_clone = entry.path()?.clone();
            let path = path_clone.to_str().unwrap().to_string();

            let mut entry_data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut entry_data)?;

            let desc = String::from_utf8(entry_data).unwrap();
            let package = Self::parse_entry_desc(desc)?;

            packages.insert(package.name.clone(), package);
        }

        Ok(Database {
            file: pth,
            signed: false,
            packages: Box::from(HashMap::new()),
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let db = if cli.db.is_some() {
        Some(cli.db.as_deref().unwrap().to_path_buf())
    } else {
        None
    };

    let pst = ProgState {
        db,
        debug_lvl: cli.debug.unwrap_or(0),
    };

    match &cli.command {
        Some(Commands::PkgInfo { pkg_name }) => {
            println!("Lookup package {}", pkg_name);
            lookup(pkg_name.into(), &pst)?;
        }
        None => println!("This tool requires a subcommand, call with -h to see options"),
    }
    
    Ok(())
}

fn lookup(pkg_name: String, pst: &ProgState) -> Result<()> {
    let db = Database::open(pst.db.clone().unwrap().clone())?;
    let pkg = db.packages.get(&pkg_name);
    if let Some(package) = pkg {
        
    }
    
    Ok(())
}
