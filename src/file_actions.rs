use anyhow::Result;
use std::path::{Path, PathBuf};

use futures::future::{BoxFuture, FutureExt};
use tokio::fs;

pub fn check_path(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }
    if path
        .metadata()
        .map(|x| x.permissions().readonly())
        .unwrap_or(true)
    {
        anyhow::bail!("Invalid permissions for file: {}", path.display());
    }

    Ok(path.to_path_buf())
}
pub fn recurse_copy<'a>(src: &'a Path, output_dest: &'a Path) -> BoxFuture<'a, Result<()>> {
    async move {
        fs::create_dir(&output_dest).await?;
        let mut files = fs::read_dir(src).await?;
        while let Some(entry) = files.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                fs::copy(&path, output_dest.join(&path.file_name().unwrap())).await?;
            } else {
                recurse_copy(&path, &output_dest.join(&path.file_name().unwrap())).await?;
            }
        }
        Ok(())
    }
    .boxed()
}
