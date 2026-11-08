use std::path::Path;

use tower_http::services::{ServeDir, ServeFile};

/// Serves the built webapp. A path that matches no file gets `index.html`, so
/// that the client-side router resolves it.
pub fn service(dir: &Path) -> ServeDir<ServeFile> {
    let index = dir.join("index.html");
    if index.is_file() {
        tracing::debug!(dir = %dir.display(), "serving the webapp");
    } else {
        tracing::warn!(dir = %dir.display(), "no index.html in the webapp directory: pages answer 404");
    }
    ServeDir::new(dir).fallback(ServeFile::new(index))
}
