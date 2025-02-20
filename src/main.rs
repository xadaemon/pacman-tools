use clap::{Parser, Subcommand};
use std::io::Read;
use std::path::Path;
use std::{io, path::PathBuf};
use thiserror::Error;

mod db;
mod actions;

use db::database::Database;
use crate::actions::lookup;

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
    #[error("An io Error occurred {0}")]
    EIO(#[from] io::Error),
    #[error("File {0} not found")]
    ENOF(PathBuf),
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
            for k in db.packages().keys() {
                println!("{}", k);
            }
        }
        None => {
            println!("This tool requires a subcommand, call with -h to see options");
            
        },
    }

    Ok(())
}
