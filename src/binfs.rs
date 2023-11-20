use std::path::PathBuf;

/// Binary FS.
///
/// This filesystem is "mounted" read-only under /bin/. All files are owned by the root user/group.
pub struct BinFs {
    mount_point: PathBuf,
}

impl BinFs {
    pub fn new(mount_point: &str) -> Self {
        Self {
            mount_point: PathBuf::from(mount_point),
        }
    }

    /// Resolve a file path to a location backing the file.
    pub fn resolve(&self, file_path: &str) -> Option<PathBuf> {
        let path_buf = PathBuf::from(file_path);

        if !path_buf.starts_with(&self.mount_point) {
            // Not under our mount point, don't try to resolve.
            return None;
        }

        if let Some(file_name) = path_buf.file_name().and_then(|name| name.to_str()) {
            Some(path_buf.with_file_name(format!("{}{}", file_name, ".js")))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve() {
        let fs = BinFs::new("/bin");

        let file_paths = vec![
            ("/bin/ls", Some(PathBuf::from("/bin/ls.js"))),
            ("/bin/perl/cpan", Some("/bin/perl/cpan.js".into())),
            ("/bin/ld.gold", Some("/bin/ld.gold.js".into())),
            ("/bin/node.js", Some("/bin/node.js.js".into())),
            ("/usr/bin/go", None),
            ("/sbin/sudo", None),
        ];

        for (input_path, expected_output) in file_paths {
            assert_eq!(fs.resolve(input_path), expected_output);
        }
    }
}
