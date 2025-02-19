use clap::builder::Str;
use clap::{Parser, Subcommand};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read};
use std::{collections::HashMap, io, path::PathBuf};
use std::path::Path;
use thiserror::Error;
use zstd::bulk::Decompressor;
use log::warn;

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
    PkgInfo {
        pkg_name: String,
    },
    List {},
}

#[derive(Debug)]
struct State {
    db: Option<Box<Path>>,
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
    metadata: HashMap<String, Vec<String>>,
}

#[derive(Debug)]
struct Database {
    file: PathBuf,
    signed: bool,
    packages: HashMap<String, Package>,
}

impl Database {
    fn open(pth: &Path) -> Result<Self> {
        if !pth.try_exists()? {
            return Err(ProgramError::ENOF(pth.to_path_buf()))
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let pst = State {
        db: if cli.db.is_some() {
            Some(Box::from(cli.db.unwrap().as_path()))
        } else {
            None
        },
        debug_lvl: cli.debug.unwrap_or(0),
    };

    match &cli.command {
        Some(Commands::PkgInfo { pkg_name }) => {
            lookup(pkg_name.into(), &pst)?;
        }
        Some(Commands::List {}) => {
            let db = Database::open(pst.db.unwrap().as_ref())?;
            for (k, _) in db.packages {
                println!("{}", k);
            }
        }
        None => println!("This tool requires a subcommand, call with -h to see options"),
    }

    Ok(())
}

fn lookup(pkg_name: String, pst: &State) -> Result<()> {
    let dbs = if pst.db.is_none() {
        // List all db files in /var/lib/pacman/sync
        let path = PathBuf::from("/var/lib/pacman/sync");
        let entries = path.read_dir()?;
        let mut dbs = Vec::new();
        for entry in entries {
            let entry = entry?;

            let metadata = entry.metadata()?;
            if !metadata.is_file() || entry.path().extension().unwrap() != "db" {
                continue;
            }

            dbs.push(entry.path());
        }
        dbs
    } else {
        vec![pst.db.clone().unwrap().to_path_buf()]
    };

    for db_file in dbs {
        let db = match Database::open(&db_file) {
            Ok(db) => db,
            Err(e) => {
                println!("Error {} at db {}", e, db_file.display());
                return Err(e)
            }
        };
        let pkg = db.packages.get(&pkg_name);
        if let Some(package) = pkg {
            println!("Found package {} in db {}", pkg_name, db_file.display());
            for (k, v) in &package.metadata {
                println!("{}, {:?}", k, v);
            }
            return Ok(())
        } 
    }
    println!("Package not found");
    Ok(())
}
