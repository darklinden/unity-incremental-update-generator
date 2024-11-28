use anyhow::Result;
use clap::Parser;
use folder_hash_list::folder_hash_list;
use giu_config::GIUConfig;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

mod file_check;
mod run_unity_build;
use run_unity_build::run_unity_build;
mod win_cyg;
use win_cyg::cyg_to_win;
mod git_cmd;
use git_cmd::{
    export_file_in_git_by_tag, get_git_tags, git_commit_with_tag, is_git_repo, is_git_repo_clean,
};
mod file_zip;
mod folder_hash_list;
mod giu_config;
mod log_util;

/// Generate incremental updates via Git Tags
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Unity Project Folder Path
    #[arg(short, long)]
    project_path: String,
}

fn load_file_hash_map(file: &Path) -> Result<HashMap<String, String>> {
    let file_content = fs::read_to_string(file)?;
    let mut map = HashMap::new();
    for line in file_content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 2 {
            continue;
        }
        let file_name = parts[0].trim();
        let hash = parts[1].trim();
        map.insert(file_name.to_string(), hash.to_string());
    }

    Ok(map)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PatchInfo {
    pub ver: String,
    pub down: String,
    pub size: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PlatformPatchInfo {
    // latest version
    pub ver: String,
    // latest version full package download name
    pub down: String,
    pub size: String,

    // incremental package versions
    pub vers: Vec<String>,
    // incremental package download name match with versions
    pub downs: Vec<String>,
    // incremental package sizes match with versions
    pub sizes: Vec<String>,
}

async fn generate_incremental_updates() -> Result<()> {
    let args: Args = Args::parse();
    let project_path = cyg_to_win(&args.project_path);
    let project_path = std::path::absolute(Path::new(&project_path))?;
    println!("project_path: {:?}", project_path);

    if !project_path.is_dir() {
        return Err(anyhow::anyhow!("Invalid project path"));
    }

    let _guards = log_util::init(&project_path);

    if !is_git_repo(&project_path).await? {
        return Err(anyhow::anyhow!("project folder is not in Git repository"));
    }

    if !is_git_repo_clean(&project_path).await? {
        return Err(anyhow::anyhow!("project folder has uncommitted changes"));
    }

    let giu_config = project_path.join(".giu_config.toml");
    if !giu_config.is_file() {
        // create default giu_config
        let default_giu_config = GIUConfig {
            unity_path: "/path/to/unity".to_string(),
            platforms: vec!["Android".to_string(), "iOS".to_string()],
        };
        let default_giu_config_str = toml::to_string(&default_giu_config)?;
        fs::write(&giu_config, default_giu_config_str)?;
        return Err(anyhow::anyhow!(
            "No .giu_config file found, created default one"
        ));
    }
    let giu_config_content = fs::read_to_string(giu_config)?;
    let giu_config: GIUConfig = toml::from_str(&giu_config_content)?;

    let unity_path = cyg_to_win(&giu_config.unity_path);
    let unity_path = Path::new(&unity_path);
    if !unity_path.is_file() {
        return Err(anyhow::anyhow!(
            "Unity path not configured in .giu_config.toml"
        ));
    }

    let platforms = giu_config.platforms;
    if platforms.is_empty() {
        return Err(anyhow::anyhow!("No platforms found in .giu_config.toml"));
    }

    let loader_version = project_path
        .join("Assets")
        .join("Resources")
        .join("Version.txt");
    if !loader_version.is_file() {
        return Err(anyhow::anyhow!(
            "No Assets/Resources/Version.txt found in project"
        ));
    }
    let loader_version = fs::read_to_string(loader_version)?.trim().to_string();

    let tags = get_git_tags(&project_path, &loader_version).await?;

    tracing::info!(
        "Loader version: {}, Will generate incremental updates for tags:",
        loader_version,
    );
    for tag in tags.iter() {
        let tag_info = git_cmd::get_git_tag_info(&project_path, tag).await?;
        tracing::info!("    {}", tag_info);
    }

    let patch_version = if tags.is_empty() {
        tracing::info!("No tags found for loader version: {}, ", loader_version);
        0
    } else {
        let last_tag = tags.first().unwrap();
        let patch_version = last_tag.split('-').last().unwrap().parse::<u32>().unwrap();
        tracing::info!("Last tag: {}, patch version: {}", last_tag, patch_version);
        patch_version + 1
    };

    // unity-project-folder/../host/serve/loader-version
    let patches_path = project_path
        .parent()
        .unwrap()
        .join("host")
        .join("serve")
        .join(&loader_version);
    fs::create_dir_all(&patches_path)?;

    for platform in platforms.iter() {
        if !run_unity_build(unity_path, &project_path, platform).await? {
            return Err(anyhow::anyhow!(format!(
                "Failed to exec Unity build for {}",
                platform
            )));
        }

        let platform_folder = project_path.join("ServerData").join(platform);

        // generate Version.txt
        let version_file = platform_folder.join("Version.txt");
        fs::write(&version_file, format!("{}", patch_version))?;

        // generate file hash list
        let hashes = folder_hash_list(&platform_folder).await?;
        let hash_file = platform_folder.join("file-hash.csv");
        fs::write(&hash_file, hashes)?;

        let new_file_hash_map = load_file_hash_map(&hash_file)?;

        // generate incremental updates
        let platform_patches_path = patches_path.join(platform);
        fs::create_dir_all(&platform_patches_path)?;

        let mut platform_patch_info_map = HashMap::new();

        if !tags.is_empty() {
            for tag in tags.iter() {
                tracing::info!(
                    "Generating incremental updates for platform: {} patch for: {}",
                    platform,
                    tag
                );
                let tag_folder = platform_patches_path.join(tag);
                fs::create_dir_all(&tag_folder)?;

                export_file_in_git_by_tag(
                    &project_path,
                    tag,
                    hash_file.to_str().unwrap(),
                    &tag_folder,
                )
                .await?;

                // the file in archive.zip is relative to the project folder
                let tag_file_hash_map = load_file_hash_map(
                    &tag_folder
                        .join("ServerData")
                        .join(platform)
                        .join("file-hash.csv"),
                )?;

                let mut diff_files = Vec::new();
                let mut diff_file_list = String::new();
                for (file_name, new_hash) in new_file_hash_map.iter() {
                    let old_hash = tag_file_hash_map.get(file_name);
                    if old_hash.is_none() || old_hash.unwrap() != new_hash {
                        diff_files.push(file_name);
                        diff_file_list.push_str(&format!("{},{}\n", file_name, new_hash));
                    }
                }

                let diff_file_list_name = format!("diff-{}.csv", tag);
                let diff_file = platform_folder.join(&diff_file_list_name);
                fs::write(&diff_file, diff_file_list)?;
                diff_files.push(&diff_file_list_name);

                // copy catalog
                let catalog_file = format!("catalog_{}.json", loader_version);
                diff_files.push(&catalog_file);
                // copy catalog hash file
                let catalog_hash_file = format!("catalog_{}.hash", loader_version);
                diff_files.push(&catalog_hash_file);
                // copy version file
                let version_file = "Version.txt".to_string();
                diff_files.push(&version_file);

                let patch_file = platform_patches_path.join(format!("{}.zip", tag));
                tracing::info!("Generated patch for tag: {}", tag);
                file_zip::compress(&platform_folder, &diff_files, &patch_file, false)?;

                let file_size = fs::metadata(patch_file)?.len();
                let file_size = file_size as f64 / 1048576_f64; // 1024 x 1024
                let file_size = if file_size < 0.01 {
                    format!("{:.2} K", file_size * 1024_f64)
                } else {
                    format!("{:.2} M", file_size)
                };
                let patch_info = PatchInfo {
                    ver: patch_version.to_string(),
                    down: format!("{}.zip", tag),
                    size: file_size,
                };
                platform_patch_info_map.insert(tag.to_string(), patch_info);
            }
        }

        tracing::info!(
            "Generating incremental updates for platform: {} full patch",
            platform
        );

        let mut diff_files = new_file_hash_map.keys().collect::<Vec<_>>();

        // copy catalog
        let catalog_file = format!("catalog_{}.json", loader_version);
        diff_files.push(&catalog_file);
        // copy catalog hash file
        let catalog_hash_file = format!("catalog_{}.hash", loader_version);
        diff_files.push(&catalog_hash_file);
        // copy version file
        let version_file = "Version.txt".to_string();
        diff_files.push(&version_file);

        let full_patch_file_name = format!("{}-full.zip", patch_version);
        let patch_file = platform_patches_path.join(&full_patch_file_name);
        tracing::info!("Generated patch full");
        file_zip::compress(&platform_folder, &diff_files, &patch_file, false)?;

        let full_file_size = fs::metadata(patch_file)?.len();
        let full_file_size = full_file_size as f64 / 1048576_f64; // 1024 x 1024
        let full_file_size = if full_file_size < 0.01 {
            format!("{:.2} K", full_file_size * 1024_f64)
        } else {
            format!("{:.2} M", full_file_size)
        };

        let mut platform_patch_info = PlatformPatchInfo {
            ver: patch_version.to_string(),
            down: full_patch_file_name,
            size: full_file_size,
            vers: Vec::new(),
            downs: Vec::new(),
            sizes: Vec::new(),
        };

        for (tag, patch_info) in platform_patch_info_map.iter() {
            platform_patch_info.vers.push(tag.to_string());
            platform_patch_info.downs.push(patch_info.down.to_string());
            platform_patch_info.sizes.push(patch_info.size.to_string());
        }

        let platform_patch_info_file = platform_patches_path.join("update_info");
        let platform_patch_info_str = serde_json::to_string(&platform_patch_info)?;
        fs::write(&platform_patch_info_file, platform_patch_info_str)?;
    }

    tracing::info!("Incremental updates generated successfully");

    git_commit_with_tag(
        &project_path,
        &format!("{}-{}", loader_version, patch_version),
        &format!("build: {}-{}", loader_version, patch_version),
    )
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    match generate_incremental_updates().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("{}", e);
            println!("Error: {}", e);
        }
    }
}
