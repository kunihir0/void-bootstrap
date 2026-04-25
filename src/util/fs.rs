use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

pub(crate) fn validate_block_device(path: &str) -> Result<()> {
    let meta = fs::metadata(path).with_context(|| format!("Path does not exist: {path}"))?;

    if !meta.file_type().is_block_device() {
        anyhow::bail!("{path} is not a valid block device");
    }
    Ok(())
}

pub(crate) fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    fs::create_dir_all(dst)?;
    fs::set_permissions(dst, fs::metadata(src)?.permissions())?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(entry.path(), dest_path)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(entry.path())?;
            fs::remove_file(&dest_path).ok();
            std::os::unix::fs::symlink(target, dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
