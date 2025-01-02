use std::path::Path;

pub(crate) async fn run_unity_build(
    unity_path: &Path,
    project_path: &Path,
    platform: &str,
    build_app: bool,
) -> anyhow::Result<bool> {
    use std::fs;
    use std::process::Stdio;
    use tokio::process::Command;
    use tokio::spawn;

    tracing::info!("unity_path: {}", unity_path.display());
    if !unity_path.is_file() && !unity_path.is_dir() {
        return Err(anyhow::anyhow!("unity path not found"));
    }
    tracing::info!("project_path: {}", project_path.display());
    if !project_path.is_dir() {
        return Err(anyhow::anyhow!("project path not found"));
    }

    let addrs_cache = project_path.join("Library/com.unity.addressables");
    if addrs_cache.is_dir() {
        fs::remove_dir_all(addrs_cache)?;
    }
    let server_data_platform = project_path.join("ServerData").join(platform);
    if server_data_platform.is_dir() {
        fs::remove_dir_all(server_data_platform)?;
    }

    tracing::info!("start building - {}", platform);

    let log_file = project_path.join("output.txt");
    if log_file.is_file() {
        fs::remove_file(&log_file)?;
    }

    spawn(async move {
        // pin!(log_file);
        // every second check if build log file increased
        let mut line_no = 0;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            if !log_file.is_file() {
                continue;
            }
            let log_file_content = fs::read_to_string(&log_file).unwrap();
            let lines = log_file_content.lines();

            let new_line_no = lines.count();
            if new_line_no > line_no {
                // print new lines
                for (i, line) in log_file_content.lines().enumerate().skip(line_no) {
                    tracing::info!("[{}] {}", i, line);
                }
                line_no = new_line_no;
            }
        }
    });

    Command::new(unity_path)
        .args([
            "-quit",
            "-batchmode",
            "-nographics",
            "-projectPath",
            project_path.to_str().unwrap(),
            "-buildTarget",
            platform,
            "-executeMethod",
            if build_app {
                "BuildDLLAndAddrs.ReleaseMainPackage"
            } else {
                "BuildDLLAndAddrs.ReleaseIncrementalServerData"
            },
            "-logFile",
            project_path.join("output.txt").to_str().unwrap(),
        ])
        .stdout(Stdio::inherit())
        .status()
        .await?;

    let output = fs::read_to_string(project_path.join("output.txt"))?;
    Ok(
        if output.contains(if build_app {
            "ReleaseMainPackage Build Success"
        } else {
            "ReleaseIncrementalServerData Build Success"
        }) {
            tracing::info!("build success - {}", platform);
            true
        } else {
            tracing::info!("build failed - {}", platform);
            false
        },
    )
}

mod test {

    #[tokio::test]
    async fn test() {
        let unity_path = "C:/Program Files/Unity/Hub/Editor/2021.3.45f1/Editor/Unity.exe";
        let unity_path = std::path::Path::new(unity_path);
        let project_path = "C:/Github/unity-hot-update/unity";
        let project_path = std::path::Path::new(project_path);
        let platform = "Android";
        match super::run_unity_build(unity_path, project_path, platform, true).await {
            Ok(_) => {}
            Err(e) => tracing::error!("error: {}", e),
        }
    }
}
