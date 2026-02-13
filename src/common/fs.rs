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

pub fn copy_dir_recursive<P1, P2>(src: P1, dst: P2, options: &CopyOptions) -> std::io::Result<()>
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
        copy_entry(&entry, dst_ref, options)?;
    }

    Ok(())
}

fn copy_entry(entry: &fs::DirEntry, dst: &Path, options: &CopyOptions) -> std::io::Result<()> {
    let entry_path = entry.path();
    let file_name = entry.file_name();

    if should_exclude(&file_name, &options.exclude) {
        return Ok(());
    }

    let dst_path = dst.join(&file_name);

    if entry_path.is_dir() {
        copy_directory(&entry_path, &dst_path, options)?;
    } else {
        copy_file(&entry_path, &dst_path)?;
    }

    Ok(())
}

fn should_exclude(file_name: &std::ffi::OsStr, exclude_list: &[String]) -> bool {
    exclude_list
        .iter()
        .any(|excluded| file_name.to_str() == Some(excluded.as_str()))
}

fn copy_directory<P1, P2>(src: P1, dst: P2, options: &CopyOptions) -> std::io::Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    fs::create_dir_all(dst.as_ref())?;
    copy_dir_recursive(src, dst, options)
}

fn copy_file<P1, P2>(src: P1, dst: P2) -> std::io::Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    fs::copy(src.as_ref(), dst.as_ref())?;
    Ok(())
}
