use crate::Result;
use crate::State;
use pacman_db_util::database::Database;
use std::path::PathBuf;

fn db_list(pst: &State) -> Result<Vec<PathBuf>> {
    if pst.db.is_none() {
        // List all db files in /var/lib/pacman/sync
        let path = if let Some(dir) = &pst.db_dir {
            PathBuf::from(dir.clone())
        } else {
            PathBuf::from("/var/lib/pacman/sync")
        };
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
        Ok(dbs)
    } else {
        Ok(vec![pst.db.clone().unwrap().to_path_buf()])
    }
}

pub fn lookup(pkg_name: String, key_names: &Option<Vec<String>>, pst: &State) -> Result<()> {
    let dbs = db_list(pst)?;

    for db_file in dbs {
        let db = match Database::open(&db_file) {
            Ok(db) => db,
            Err(e) => {
                println!("Error {} at db {}", e, db_file.display());
                return Err(e);
            }
        };
        let pkg = db.lookup(&pkg_name);
        if let Some(package) = pkg {
            if let Some(keys) = key_names {
                for k in keys {
                    if let Some(lines) = package.metadata().get(k) {
                        for line in lines {
                            println!("{}", line);
                        }
                    }
                }
            } else {
                println!("Found package {} in db {}", pkg_name, db_file.display());
                for (k, v) in package.metadata() {
                    println!("{}, {:?}", k, v);
                }
            }
            return Ok(());
        }
    }
    println!("Package not found");
    Ok(())
}

pub fn list_all(pst: &State) -> Result<()> {
    let dbs = db_list(pst)?;

    for db_file in dbs {
        let db = Database::open(&db_file)?;
        println!(
            "database: {}",
            db.file().file_name().unwrap().to_string_lossy()
        );
        for pkg in db.packages().keys() {
            println!("{}", pkg);
        }
    }

    Ok(())
}

#[cfg(feature = "serde-types")]
pub fn all_to_json(pst: &State) -> Result<()> {
    let dbs = db_list(pst)?;
    let mut pkgs = Vec::<Database>::new();
    for db_file in dbs {
        let db = Database::open(&db_file)?;
        pkgs.push(db);
    }

    println!("{}", serde_json::to_string(&pkgs).unwrap());

    Ok(())
}
