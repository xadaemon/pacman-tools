use std::path::PathBuf;
use crate::db::database::Database;
use crate::State;
use crate::Result;

fn db_list(pst: &State) -> Result<Vec<PathBuf>> {
    if pst.db.is_none() {
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
        Ok(dbs)
    } else {
        Ok(vec![pst.db.clone().unwrap().to_path_buf()])
    }
}

pub fn lookup(pkg_name: String, pst: &State) -> crate::Result<()> {
    let dbs = db_list(pst)?;
    
    for db_file in dbs {
        let db = match Database::open(&db_file) {
            Ok(db) => db,
            Err(e) => {
                println!("Error {} at db {}", e, db_file.display());
                return Err(e)
            }
        };
        let pkg = db.lookup(&pkg_name);
        if let Some(package) = pkg {
            println!("Found package {} in db {}", pkg_name, db_file.display());
            for (k, v) in package.metadata() {
                println!("{}, {:?}", k, v);
            }
            return Ok(())
        }
    }
    println!("Package not found");
    Ok(())
}
