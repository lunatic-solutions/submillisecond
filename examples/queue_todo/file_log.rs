use std::{
    fs::{DirBuilder, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dtos::{Todo, User};
// =====================================
// Persistence utils
// =====================================
const NEWLINE: &[u8] = &[b'\n'];

/// Every Line is a new "state change" entry
/// Each line starts with one of the following keywords
/// that indicate the type of entry
const NEW_USER: u8 = 1;
const PUSH_TODO: u8 = 2;
const POLL_TODO: u8 = 3;

#[derive(Debug)]
pub struct FileLog {
    // cwd: str,
    // file_name: str,
    full_path: PathBuf,
    file: File,
}

#[derive(Serialize, Deserialize)]
struct PushEntry {
    user_uuid: Uuid,
    todo: Todo,
}

impl FileLog {
    pub fn new(cwd: &str, file_name: &str) -> FileLog {
        DirBuilder::new().recursive(true).create(cwd).unwrap();
        let full_path = Path::new(cwd).join(file_name);
        FileLog {
            // cwd,
            // file_name,
            full_path: full_path.to_path_buf(),
            file: match File::create(&full_path) {
                Err(why) => panic!("couldn't open {:?}: {}", cwd, why),
                // write 0 as initial cursor
                Ok(file) => file,
            },
        }
    }

    pub fn append_new_user(&mut self, user: &User) {
        self.append(NEW_USER, ron::to_string(user).unwrap().as_bytes())
    }

    pub fn append_poll_todo(&mut self, user_uuid: Uuid, todo_uuid: Uuid) {
        self.append(
            POLL_TODO,
            ron::to_string(&(user_uuid, todo_uuid)).unwrap().as_bytes(),
        )
    }

    pub fn append_push_todo(&mut self, user_uuid: Uuid, todo: Todo) {
        self.append(
            PUSH_TODO,
            ron::to_string(&PushEntry { user_uuid, todo })
                .unwrap()
                .as_bytes(),
        )
    }

    pub fn append(&mut self, header: u8, data: &[u8]) {
        // let x: MyStruct = ron::from_str("(boolean: true, float: 1.23)").unwrap();
        let encoded = base64::encode(data);
        let buf = [&[header], encoded.as_bytes(), NEWLINE].concat();
        match self.file.write_all(&buf) {
            Err(why) => panic!(
                "[FileLog {:?}] couldn't write to file: {}",
                self.full_path, why
            ),
            Ok(_) => println!(
                "[FileLog {:?}] Successfully appended log to file",
                self.full_path
            ),
        };
    }
}
