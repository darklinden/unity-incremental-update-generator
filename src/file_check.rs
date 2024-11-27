pub(crate) async fn file_hash(path: &std::path::Path) -> anyhow::Result<String> {
    use crc32fast::Hasher;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    let mut hasher = Hasher::new();
    let mut file = File::open(path).await?;
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer[..]).await?;

        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:X}", hash))
}

mod test {

    #[tokio::test]
    async fn test() {
        let unity_path = "C:/Program Files/Unity/Hub/Editor/2021.3.45f1/Editor/Unity.exe";
        let unity_path = std::path::Path::new(unity_path);
        let meta = std::fs::metadata(unity_path).unwrap();
        tracing::info!("meta: {:#?}", meta.len());
        match super::file_hash(unity_path).await {
            Ok(hash) => {
                tracing::info!("hash: {}", hash);
            }
            Err(e) => tracing::error!("error: {}", e),
        }
    }
}
