use std::io::SeekFrom;
use std::path::Path;

use axum::body::Body;
use axum::http::header::{
    ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
};
use axum::http::StatusCode;
use axum::response::Response;
use droply_domain::{Download, DroplyError};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

/// Single-range subset of RFC 7233 — enough for resumable downloads and
/// video seeking (the doc's stated reasons for range support, §26), not a
/// full implementation (no multi-range `bytes=0-10,20-30` responses).
#[derive(Debug, PartialEq, Eq)]
enum RangeRequest {
    /// No `Range` header, or one that isn't a `bytes=` range — serve the
    /// whole file. Per RFC 7233 a server may ignore a header it doesn't
    /// understand rather than rejecting the request.
    Full,
    Partial {
        start: u64,
        end: u64,
    },
    Unsatisfiable,
}

fn parse_range(header: Option<&str>, file_size: u64) -> RangeRequest {
    let Some(header) = header else {
        return RangeRequest::Full;
    };
    let Some(value) = header.strip_prefix("bytes=") else {
        return RangeRequest::Full;
    };
    // Multi-range requests aren't supported — fall back to the whole file
    // rather than reject the request outright.
    if value.contains(',') {
        return RangeRequest::Full;
    }
    let Some((start_str, end_str)) = value.split_once('-') else {
        return RangeRequest::Full;
    };

    if file_size == 0 {
        return RangeRequest::Unsatisfiable;
    }

    if start_str.is_empty() {
        // Suffix range: "-500" means the last 500 bytes.
        let Ok(suffix_length) = end_str.parse::<u64>() else {
            return RangeRequest::Full;
        };
        if suffix_length == 0 {
            return RangeRequest::Unsatisfiable;
        }
        let start = file_size.saturating_sub(suffix_length);
        return RangeRequest::Partial {
            start,
            end: file_size - 1,
        };
    }

    let Ok(start) = start_str.parse::<u64>() else {
        return RangeRequest::Full;
    };
    if start >= file_size {
        return RangeRequest::Unsatisfiable;
    }

    let end = if end_str.is_empty() {
        file_size - 1
    } else {
        match end_str.parse::<u64>() {
            Ok(end) => end.min(file_size - 1),
            Err(_) => return RangeRequest::Full,
        }
    };

    if end < start {
        return RangeRequest::Unsatisfiable;
    }

    RangeRequest::Partial { start, end }
}

/// A `Content-Disposition` filename value must not contain a raw `"` or
/// control characters — `Download::file_name` is already sanitized by
/// `derive_filename` at analysis time, but this is the actual HTTP
/// response, so it gets its own narrow defense rather than trusting that
/// upstream guarantee to hold forever.
fn sanitize_header_value(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() && *c != '"')
        .collect()
}

pub async fn serve_file(
    path: &Path,
    download: &Download,
    range_header: Option<&str>,
) -> Result<Response, DroplyError> {
    let mut file = File::open(path)
        .await
        .map_err(|_| DroplyError::SourceUnavailable)?;
    let file_size = file
        .metadata()
        .await
        .map_err(|_| DroplyError::SourceUnavailable)?
        .len();

    let content_type = download
        .media_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let disposition = format!(
        "attachment; filename=\"{}\"",
        sanitize_header_value(&download.file_name)
    );

    match parse_range(range_header, file_size) {
        RangeRequest::Unsatisfiable => {
            let response = Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(CONTENT_RANGE, format!("bytes */{file_size}"))
                .body(Body::empty())
                .map_err(build_error)?;
            Ok(response)
        }
        RangeRequest::Full => {
            let stream = ReaderStream::new(file);
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, content_type)
                .header(CONTENT_DISPOSITION, disposition)
                .header(CONTENT_LENGTH, file_size)
                .header(ACCEPT_RANGES, "bytes")
                .body(Body::from_stream(stream))
                .map_err(build_error)?;
            Ok(response)
        }
        RangeRequest::Partial { start, end } => {
            file.seek(SeekFrom::Start(start))
                .await
                .map_err(|_| DroplyError::SourceUnavailable)?;
            let length = end - start + 1;
            let stream = ReaderStream::new(file.take(length));
            let response = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(CONTENT_TYPE, content_type)
                .header(CONTENT_DISPOSITION, disposition)
                .header(CONTENT_LENGTH, length)
                .header(CONTENT_RANGE, format!("bytes {start}-{end}/{file_size}"))
                .header(ACCEPT_RANGES, "bytes")
                .body(Body::from_stream(stream))
                .map_err(build_error)?;
            Ok(response)
        }
    }
}

fn build_error(_: axum::http::Error) -> DroplyError {
    DroplyError::ProcessingFailed {
        reason: "failed to build file response".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::RangeRequest::*;
    use super::*;

    #[test]
    fn no_header_means_full_content() {
        assert_eq!(parse_range(None, 1000), Full);
    }

    #[test]
    fn non_bytes_unit_falls_back_to_full() {
        assert_eq!(parse_range(Some("items=0-5"), 1000), Full);
    }

    #[test]
    fn simple_bounded_range() {
        assert_eq!(
            parse_range(Some("bytes=0-499"), 1000),
            Partial { start: 0, end: 499 }
        );
    }

    #[test]
    fn open_ended_range_goes_to_end_of_file() {
        assert_eq!(
            parse_range(Some("bytes=500-"), 1000),
            Partial {
                start: 500,
                end: 999
            }
        );
    }

    #[test]
    fn suffix_range_takes_the_last_n_bytes() {
        assert_eq!(
            parse_range(Some("bytes=-100"), 1000),
            Partial {
                start: 900,
                end: 999
            }
        );
    }

    #[test]
    fn suffix_range_larger_than_file_clamps_to_whole_file() {
        assert_eq!(
            parse_range(Some("bytes=-5000"), 1000),
            Partial { start: 0, end: 999 }
        );
    }

    #[test]
    fn end_beyond_file_size_is_clamped() {
        assert_eq!(
            parse_range(Some("bytes=0-999999"), 1000),
            Partial { start: 0, end: 999 }
        );
    }

    #[test]
    fn start_at_or_past_file_size_is_unsatisfiable() {
        assert_eq!(parse_range(Some("bytes=1000-"), 1000), Unsatisfiable);
        assert_eq!(parse_range(Some("bytes=5000-6000"), 1000), Unsatisfiable);
    }

    #[test]
    fn multi_range_requests_fall_back_to_full() {
        assert_eq!(parse_range(Some("bytes=0-10,20-30"), 1000), Full);
    }

    #[test]
    fn garbage_falls_back_to_full() {
        assert_eq!(parse_range(Some("bytes=abc-def"), 1000), Full);
        assert_eq!(parse_range(Some("nonsense"), 1000), Full);
    }

    #[test]
    fn sanitizes_quotes_and_control_characters_from_filenames() {
        let cleaned = sanitize_header_value("evil\"; filename=\"other.txt");
        assert!(!cleaned.contains('"'));
    }
}
