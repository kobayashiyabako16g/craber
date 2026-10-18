//! The site as a browser asks for it: files keyed by URL path.

use std::collections::HashMap;

use craber_site::SiteFile;

/// A ready-to-serve site: every file keyed by the path a browser will ask for.
pub struct Routes(HashMap<String, SiteFile>);

impl Routes {
    /// `index.html` is reachable both by name and as `/`, which is what a
    /// browser actually requests.
    pub(super) fn build(files: Vec<SiteFile>) -> Self {
        let mut routes = HashMap::with_capacity(files.len() + 1);
        for file in files {
            let path = file.path.to_string_lossy().replace('\\', "/");
            if path == "index.html" {
                routes.insert("/".to_owned(), file.clone());
            }
            routes.insert(format!("/{path}"), file);
        }
        Self(routes)
    }

    /// The file at `path`, or `None` for a miss.
    ///
    /// Paths are keys in a map built from the embedded bundle, never joined
    /// onto a filesystem path, so `../` is simply a miss. There is no traversal
    /// to defend against.
    pub(super) fn get(&self, path: &str) -> Option<&SiteFile> {
        self.0.get(path)
    }
}
