use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn create_unique_child_folder(parent: &Path) -> Result<PathBuf, String> {
    if !parent.is_dir() {
        return Err(format!(
            "New folder failed: parent folder {} is unavailable",
            parent.display()
        ));
    }
    for index in 1.. {
        let name = if index == 1 {
            String::from("New Folder")
        } else {
            format!("New Folder {index}")
        };
        let candidate = parent.join(name);
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("New folder failed: {error}")),
        }
    }
    unreachable!("unbounded folder name search should return or fail")
}
