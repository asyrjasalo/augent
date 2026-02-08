//! Common file system operations with unified error handling

use std::fs;
use std::path::Path;

#[derive(Default, Clone)]
pub struct CopyOptions {
    pub exclude: Vec<String>,
}

impl CopyOptions {
    pub fn exclude_git() -> Self {
        Self {
            exclude: vec![".git".to_string()],
        }
    }
}

/// Copy a directory recursively with options
pub fn copy_dir_recursive<P1, P2>(src: P1, dst: P2, options: CopyOptions) -> std::io::Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let src_ref = src.as_ref();
    let dst_ref = dst.as_ref();

    if !dst_ref.exists() {
        fs::create_dir_all(dst_ref)?;
    }

    for entry in fs::read_dir(src_ref)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name();

        if options
            .exclude
            .iter()
            .any(|excluded| file_name.to_str() == Some(excluded.as_str()))
        {
            continue;
        }

        let dst_path = dst_ref.join(&file_name);

        if entry_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&entry_path, &dst_path, options.clone())?;
        } else {
            fs::copy(&entry_path, &dst_path)?;
        }
    }

    Ok(())
}
