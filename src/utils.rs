use crate::types::*;
use std::{fs, os::macos::fs::MetadataExt, path::PathBuf};
use tokio::sync::mpsc::Sender;

pub async fn scan_dir_path_task(
    dir_path: PathBuf,
    dirp_state_sender: Sender<DirpStateMessage>,
    recurse: bool,
) -> Result<Vec<FSObj>, DirpError> {
    tokio::spawn(async move {
        // Create a list containing a FSObj for each directory item in the
        // specified dir and return it.
        let mut fs_obj_list = Vec::<FSObj>::new();

        for dir_entry in fs::read_dir(dir_path.clone())? {
            let dir_entry = dir_entry?;
            let obj_path = dir_entry.path();
            let meta_data = dir_entry.metadata()?;

            if obj_path.is_dir() {
                fs_obj_list.push(FSObj::Dir(Dir::new(obj_path.clone())));
                if recurse {
                    scan_dir_path_task(obj_path, dirp_state_sender.clone(), recurse);
                }
            } else if obj_path.is_file() {
                fs_obj_list.push(FSObj::File(File::new(obj_path, meta_data.st_size())))
            } else if obj_path.is_symlink() {
                fs_obj_list.push(FSObj::Link(Link::new(obj_path)))
            }
        }

        dirp_state_sender.send(DirpStateMessage::DirScanMessage(DirScanMessage {
            dir_path,
            fs_obj_list: fs_obj_list.clone(),
        }));

        Ok(fs_obj_list)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn test_scan_dir_path_task() {
        let (sender, _receiver) = channel(1024);

        match scan_dir_path_task(PathBuf::from("./test/a/"), sender, false).await {
            Ok(fs_obj_list) => {
                let expected_result = r#"[File(File{name:"./test/a/3.txt",size_in_bytes:1010,},),File(File{name:"./test/a/2.txt",size_in_bytes:1010,},),File(File{name:"./test/a/1.txt",size_in_bytes:1010,},),]"#;

                let result = format!("{:#?}", fs_obj_list)
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>();

                assert_eq!(result, expected_result, "Unexpected result.");
            }
            Err(error) => assert!(false, "{}", format!("Error: {:#?}", error)),
        }
    }
}
