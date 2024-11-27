pub(crate) async fn folder_hash_list(folder: &std::path::Path) -> anyhow::Result<String> {
    use crate::file_check::file_hash;
    use tokio::fs;

    tracing::info!("folder: {}", folder.display());
    if !folder.is_dir() {
        return Err(anyhow::anyhow!("folder path not found"));
    }

    let mut files = fs::read_dir(folder).await?;
    let mut csv_content = String::new();
    // spawn async tasks to calculate hash of each file
    while let Ok(file) = files.next_entry().await {
        if let Some(file) = file {
            let file_path = file.path();
            if file_path.is_dir() {
                tracing::error!("skip folder: {}", file_path.display());
                continue;
            }
            if file_path.extension().unwrap() != "bundle" {
                continue;
            }
            let file_name = file_path.file_name().unwrap().to_str().unwrap();
            let file_hash = file_hash(&file_path).await?;
            csv_content.push_str(&format!("{},{}\n", file_name, file_hash));
        } else {
            break;
        }
    }

    Ok(csv_content)
}

mod test {

    #[tokio::test]
    async fn test() {
        let folder = "C:/Github/unity-incremental-update/unity/ServerData/Android";
        let folder = std::path::Path::new(folder);
        match super::folder_hash_list(folder).await {
            Ok(list) => {
                tracing::info!("list: {:#}", list);
            }
            Err(e) => tracing::error!("error: {}", e),
        }
    }
}
