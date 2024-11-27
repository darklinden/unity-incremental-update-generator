use anyhow::Result;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;

pub(crate) fn extract(src_file: &Path, des_folder: &Path) -> Result<i32> {
    let file = File::open(src_file)?;

    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = match file.enclosed_name() {
            Some(path) => des_folder.join(path),
            None => continue,
        };

        // {
        //     let comment = file.comment();
        //     if !comment.is_empty() {
        //         tracing::info!("File {i} comment: {comment}");
        //     }
        // }

        if file.is_dir() {
            tracing::info!("File {} extracted to \"{}\"", i, out_path.display());
            fs::create_dir_all(&out_path)?;
        } else {
            tracing::info!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                out_path.display(),
                file.size()
            );
            if let Some(p) = out_path.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(0)
}

pub(crate) fn compress(
    prefix: &Path,
    src_files: &[&String],
    des_file: &Path,
    append: bool,
) -> anyhow::Result<()> {
    // will only use this for zip, if use this function for other compression may cause error
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let mut zip_writer = if append {
        let existing_zip = OpenOptions::new().read(true).write(true).open(des_file)?;
        zip::ZipWriter::new_append(existing_zip)?
    } else {
        if des_file.exists() {
            fs::remove_file(des_file)?;
        }
        let file = fs::File::create(des_file)?;
        zip::ZipWriter::new(file)
    };

    let prefix = Path::new(prefix);
    let mut buffer = Vec::new();
    for name in src_files.iter() {
        let path = prefix.join(name);
        let relative = path.strip_prefix(prefix).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            tracing::info!("adding file {path:?} as {name:?} ...");
            zip_writer.start_file(relative.to_str().unwrap(), options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip_writer.write_all(&buffer)?;
            buffer.clear();
        } else if !relative.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and map name conversion failed error on unzip
            tracing::info!("adding dir {relative:?} as {name:?} ...");
            zip_writer.add_directory(relative.to_str().unwrap(), options)?;
        }
    }
    zip_writer.finish()?;
    Ok(())
}
