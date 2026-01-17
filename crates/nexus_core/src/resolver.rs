use oxc_resolver::{ResolveOptions, Resolver, Resolution};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct NexusResolver {
    resolver: Arc<Resolver>,
}

impl NexusResolver {
    pub fn new(project_root: &Path) -> Self {
        let options = ResolveOptions {
             extensions: vec![
                ".ts".into(),
                ".tsx".into(),
                ".js".into(),
                ".jsx".into(),
                ".json".into(),
             ],
             alias_fields: vec![vec!["browser".into()]], // Browser preference for some packages
             main_fields: vec!["browser".into(), "module".into(), "main".into()],
             condition_names: vec!["browser".into(), "import".into(), "require".into()],
             ..ResolveOptions::default()
        };
        
        // Resolver handles caching internally usually, or is stateless per call?
        // oxc_resolver::Resolver is constructed with options.
        let resolver = Resolver::new(options);
        
        Self {
            resolver: Arc::new(resolver),
        }
    }

    pub fn resolve(&self, from: &Path, import: &str) -> std::io::Result<PathBuf> {
        // `from` is the directory containing the file doing the import, OR the file itself?
        // oxc_resolver resolve(path, specifier). Path should be directory.
        
        let dir = if from.is_file() {
            from.parent().unwrap_or(Path::new("/"))
        } else {
            from
        };

        match self.resolver.resolve(dir, import) {
            Ok(Resolution { path, .. }) => Ok(path),
            Err(e) => {
                 // Convert oxc error to io error for simplicity or handle gracefully
                 Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Resolution failed: {}", e)))
            }
        }
    }
}
