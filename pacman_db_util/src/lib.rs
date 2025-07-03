pub mod db;
use std::{io, path::PathBuf};
use thiserror::Error;

pub use db::database;

type Result<T> = std::result::Result<T, ProgramError>;
#[derive(Error, Debug)]
pub enum ProgramError {
    #[error("An io Error occurred {0}")]
    EIO(#[from] io::Error),
    #[error("File {0} not found")]
    ENOF(PathBuf),
}
