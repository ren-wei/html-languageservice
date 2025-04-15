use std::{
    path::{Component, PathBuf},
    str::FromStr,
};

use lsp_types::{ClientCapabilities, Uri};

#[derive(Default)]
pub struct HTMLLanguageServiceOptions {
    /**
     * Unless set to false, the default HTML data provider will be used
     * along with the providers from customDataProviders.
     * Defaults to true.
     */
    pub use_default_data_provider: Option<bool>,

    /**
     * Provide data that could enhance the service's understanding of
     * HTML tag / attribute / attribute-value
     */
    // pub custom_data_providers: Option<Vec<Box<dyn IHTMLDataProvider>>>,

    /**
     * Abstract file system access away from the service.
     * Used for path completion, etc.
     */
    pub file_system_provider: Option<Box<dyn FileSystemProvider>>,

    /**
     * Describes the LSP capabilities the client supports.
     */
    pub client_capabilities: Option<ClientCapabilities>,
}

pub trait FileSystemProvider: Send + Sync {
    fn stat(&self, uri: DocumentUri) -> FileStat;

    fn read_directory(&self, uri: DocumentUri) -> (String, FileType);
}

pub type DocumentUri = String;

pub struct FileStat {
    /// The type of the file, e.g. is a regular file, a directory, or symbolic link
    /// to a file.
    pub file_type: FileType,
    /// The creation timestamp in milliseconds elapsed since January 1, 1970 00:00:00 UTC.
    pub ctime: i128,
    /// The modification timestamp in milliseconds elapsed since January 1, 1970 00:00:00 UTC.
    pub mtime: i128,
    /// The size in bytes.
    pub size: usize,
}

pub enum FileType {
    /// The file type is unknown.
    Unknown = 0,
    /// A regular file.
    File = 1,
    /// A directory.
    Directory = 2,
    /// A symbolic link to a file.
    SymbolicLink = 64,
}

pub trait DocumentContext {
    fn resolve_reference(&self, reference: &str, base: &str) -> Option<String>;
}

pub struct DefaultDocumentContext;

impl DocumentContext for DefaultDocumentContext {
    fn resolve_reference(&self, reference: &str, base: &str) -> Option<String> {
        if let Ok(uri) = Uri::from_str(reference) {
            let base_uri = Uri::from_str(base).unwrap();
            if uri.scheme().is_some() {
                return Some(uri.to_string());
            }

            let scheme = base_uri.scheme().unwrap();
            let auth = if let Some(auth) = uri.authority() {
                auth.to_string()
            } else if let Some(auth) = base_uri.authority() {
                auth.to_string()
            } else {
                "".to_string()
            };

            let mut base_uri_path = PathBuf::from_str(&base_uri.path().to_string()).unwrap();
            if !base.ends_with("/") {
                base_uri_path.pop();
            }
            let uri_path = PathBuf::from_str(&uri.path().to_string()).unwrap();
            let path = base_uri_path.join(uri_path);
            let suffix = if reference.ends_with("/") || reference.ends_with(".") {
                "/"
            } else {
                ""
            };
            let mut new_path = vec![];
            let mut components = path.components();
            let mut base_uri_components = base_uri_path.components();
            let base_prefix = {
                match base_uri_components.next() {
                    Some(Component::Prefix(preifx)) => Some(Component::Prefix(preifx)),
                    Some(Component::RootDir) => {
                        if let Some(Component::Normal(v)) = base_uri_components.next() {
                            if v.to_string_lossy().contains(":") {
                                Some(Component::Normal(v))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            };
            // first
            match components.next() {
                Some(Component::Prefix(prefix)) => new_path.push(Component::Prefix(prefix)),
                Some(Component::RootDir) => {
                    let add_prefx = if let Some(Component::Normal(v)) = components.clone().next() {
                        !v.to_string_lossy().contains(":")
                    } else {
                        true
                    };
                    if add_prefx && base_prefix.is_some() {
                        if let Some(Component::Prefix(prefix)) = base_prefix {
                            new_path.push(Component::Prefix(prefix));
                            new_path.push(Component::RootDir);
                        } else if let Some(Component::Normal(prefix)) = base_prefix {
                            new_path.push(Component::RootDir);
                            new_path.push(Component::Normal(prefix));
                        }
                    } else {
                        new_path.push(Component::RootDir);
                    }
                }
                Some(Component::Normal(v)) => {
                    if let Some(Component::Prefix(prefix)) = base_prefix {
                        new_path.push(Component::Prefix(prefix));
                        new_path.push(Component::RootDir);
                    } else if let Some(Component::Normal(prefix)) = base_prefix {
                        new_path.push(Component::RootDir);
                        new_path.push(Component::Normal(prefix));
                    } else {
                        new_path.push(Component::RootDir);
                    }
                    new_path.push(Component::Normal(v));
                }
                _ => {}
            }
            // other
            for component in components {
                match component {
                    Component::Prefix(prefix) => new_path.push(Component::Prefix(prefix)),
                    Component::RootDir => new_path.push(Component::RootDir),
                    Component::CurDir => {}
                    Component::ParentDir => {
                        new_path.pop();
                    }
                    Component::Normal(v) => new_path.push(Component::Normal(v)),
                }
            }
            let new_path = new_path.iter().collect::<PathBuf>();
            let new_path = new_path.to_string_lossy();

            Some(format!("{}://{}{}{}", scheme, auth, new_path, suffix))
        } else {
            None
        }
    }
}
