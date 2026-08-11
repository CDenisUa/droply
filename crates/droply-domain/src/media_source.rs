use serde::{Deserialize, Serialize};

/// Result of source analysis — what `docs/architecture.md` §9 calls a
/// `MediaSource`. `MediaVariant`/multi-quality selection isn't modeled yet:
/// it only matters once HLS/DASH analyzers exist (Phase 4/5); a direct file
/// is a single variant by definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceType {
    DirectFile,
    Hls,
    Dash,
    LocalFile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSourceResult {
    pub source_type: SourceType,
    pub title: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub duration_seconds: Option<f64>,
}

const MAX_FILENAME_LENGTH: usize = 255;
const FALLBACK_FILENAME: &str = "download";

/// Derive a safe filename for a direct download, preferring the server's
/// `Content-Disposition` header over the URL path — a URL path is often not
/// the real filename (query strings, redirects through signed-URL hosts,
/// extensionless paths), but a URL is the only thing guaranteed to exist.
///
/// Never trust either source directly: doc §13 ("do not trust the filename
/// extension as the primary type detector") and AGENTS.md rule 6-8 both
/// imply this value ends up as a path component on the user's device, so it
/// must not contain path separators or control characters.
pub fn derive_filename(content_disposition: Option<&str>, url_path: &str) -> String {
    let candidate = content_disposition
        .and_then(filename_from_content_disposition)
        .or_else(|| filename_from_url_path(url_path));

    match candidate {
        Some(name) => sanitize_filename(&name),
        None => FALLBACK_FILENAME.to_string(),
    }
}

fn filename_from_content_disposition(header: &str) -> Option<String> {
    for part in header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("filename*=") {
            // RFC 5987 extended notation: filename*=UTF-8''name.ext
            let decoded = value.rsplit("''").next().unwrap_or(value);
            return Some(percent_decode(decoded.trim_matches('"')));
        }
        if let Some(value) = part.strip_prefix("filename=") {
            return Some(percent_decode(value.trim_matches('"')));
        }
    }
    None
}

fn filename_from_url_path(url_path: &str) -> Option<String> {
    url_path
        .rsplit('/')
        .next()
        .map(percent_decode)
        .filter(|s| !s.is_empty())
}

/// Minimal percent-decoding — good enough for filenames, not a general URL
/// decoder (no `+`-as-space handling, no `url` crate dependency needed here
/// since `droply-domain` stays free of I/O/parsing-library dependencies).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    out.push(value);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    // Belt-and-suspenders: separators are already gone, but also collapse
    // ".." so nothing resembling a traversal sequence survives review.
    let cleaned = cleaned.replace("..", "_");

    let trimmed = cleaned.trim().trim_matches('.');

    if trimmed.is_empty() {
        return FALLBACK_FILENAME.to_string();
    }

    trimmed.chars().take(MAX_FILENAME_LENGTH).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_content_disposition_filename() {
        let name = derive_filename(
            Some(r#"attachment; filename="Movie.mp4""#),
            "/files/actual-path.bin",
        );
        assert_eq!(name, "Movie.mp4");
    }

    #[test]
    fn falls_back_to_url_path_when_no_content_disposition() {
        let name = derive_filename(None, "/downloads/clip.mov");
        assert_eq!(name, "clip.mov");
    }

    #[test]
    fn falls_back_to_default_when_nothing_usable() {
        assert_eq!(derive_filename(None, "/"), FALLBACK_FILENAME);
        assert_eq!(derive_filename(None, ""), FALLBACK_FILENAME);
        assert_eq!(derive_filename(Some("inline"), "/"), FALLBACK_FILENAME);
    }

    #[test]
    fn strips_path_separators_from_content_disposition_filenames() {
        let name = derive_filename(Some(r#"attachment; filename="../../etc/passwd""#), "/x");
        assert!(!name.contains('/'));
        assert!(!name.contains(".."));
    }

    #[test]
    fn decodes_percent_encoded_url_path_segments() {
        let name = derive_filename(None, "/files/My%20Movie%20(2026).mp4");
        assert_eq!(name, "My Movie (2026).mp4");
    }

    #[test]
    fn decodes_rfc5987_extended_filename_notation() {
        let name = derive_filename(
            Some("attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf"),
            "/x",
        );
        assert_eq!(name, "résumé.pdf");
    }

    #[test]
    fn truncates_extremely_long_filenames() {
        let long_name = "a".repeat(500);
        let header = format!(r#"attachment; filename="{long_name}.mp4""#);
        let name = derive_filename(Some(&header), "/x");
        assert!(name.chars().count() <= MAX_FILENAME_LENGTH);
    }

    #[test]
    fn rejects_control_characters() {
        let name = derive_filename(Some("attachment; filename=\"evil\nname.mp4\""), "/x");
        assert!(!name.contains('\n'));
    }
}
