use anyhow::Result;
use std::{fs, path::Path};
use tokio::process;

use crate::file_zip;

use super::win_cyg::win_to_cyg;

pub(crate) async fn is_git_repo(folder: &Path) -> Result<bool> {
    let output = process::Command::new("git")
        .current_dir(folder)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .await?;

    let is_git_repo = String::from_utf8(output.stdout)?;
    Ok(is_git_repo.trim() == "true")
}

pub(crate) async fn is_git_repo_clean(folder: &Path) -> Result<bool> {
    let output = process::Command::new("git")
        .current_dir(folder)
        .args(["status", "--porcelain"])
        .output()
        .await?;

    let is_clean = String::from_utf8(output.stdout)?;
    Ok(is_clean.trim().is_empty())
}

pub(crate) async fn get_git_tags(folder: &Path, loader_version: &str) -> Result<Vec<String>> {
    let output = process::Command::new("git")
        .current_dir(folder)
        .args(["tag", "--list"])
        .output()
        .await?;

    let git_tags = String::from_utf8(output.stdout)?;
    let git_tags = git_tags.lines();
    let mut tags = Vec::new();
    let prefix = format!("{}-", loader_version);
    for tag in git_tags {
        if tag.starts_with(&prefix) {
            tags.push(tag.to_string());
        } else {
            break;
        }
    }
    // sort by Patch version in DESC order
    tags.sort_by(|a, b| {
        let a = a.split('-').last().unwrap().parse::<u32>().unwrap();
        let b = b.split('-').last().unwrap().parse::<u32>().unwrap();
        b.cmp(&a)
    });
    Ok(tags)
}

pub(crate) async fn get_git_tag_info(folder: &Path, tag: &str) -> Result<String> {
    // commit hash, message, author, date
    let output = process::Command::new("git")
        .current_dir(folder)
        .args(["show", "--no-patch", "--format=%H %s %an %ad", tag])
        .output()
        .await?;

    let git_tag_info = String::from_utf8(output.stdout)?;
    Ok(git_tag_info.trim().to_string())
}

pub(crate) async fn export_file_in_git_by_tag(
    folder: &Path,
    tag: &str,
    file: &str,
    des_folder: &Path,
) -> Result<()> {
    fs::create_dir_all(des_folder)?;
    let des_file_path = des_folder.join("archive.zip");
    let des_file = des_file_path.to_str().unwrap();
    let des_file = win_to_cyg(des_file);
    let file = win_to_cyg(file);
    tracing::info!(
        "git archive --format=zip --output={} {} {}",
        des_file,
        tag,
        file
    );
    // git archive
    let _ = process::Command::new("git")
        .current_dir(folder)
        .args([
            "archive",
            "--format=zip",
            &format!("--output={}", des_file),
            tag,
            &file,
        ])
        .output()
        .await?;
    // unzip
    file_zip::extract(&des_folder.join("archive.zip"), des_folder)?;

    Ok(())
}

pub(crate) async fn git_commit_with_tag(folder: &Path, tag: &str, message: &str) -> Result<()> {
    // git add
    let _ = process::Command::new("git")
        .current_dir(folder)
        .args(["add", "."])
        .output()
        .await?;
    // git commit
    let _ = process::Command::new("git")
        .current_dir(folder)
        .args(["commit", "-am", message])
        .output()
        .await?;
    // git tag
    let _ = process::Command::new("git")
        .current_dir(folder)
        .args(["tag", tag])
        .output()
        .await?;

    Ok(())
}
