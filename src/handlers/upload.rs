use axum::{
    extract::Path,
    http::{StatusCode},
    response::Response,
};
use tokio_util::io::ReaderStream;
use std::path::Path as StdPath;

use crate::errors::{AppError, Result};

pub async fn serve_image(Path(file_name): Path<String>) -> Result<Response> {
    // Security: prevent path traversal
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(AppError::PostNotFound);
    }

    let file_path = format!("uploads/images/{}", file_name);

    // Check if file exists and is a file (not a directory)
    if !StdPath::new(&file_path).is_file() {
        return Err(AppError::PostNotFound);
    }

    // Try to open the file
    let file = tokio::fs::File::open(&file_path)
        .await
        .map_err(|_| AppError::PostNotFound)?;

    // Convert the file into a Stream
    let stream = ReaderStream::new(file);

    // Set appropriate content type based on file extension
    let content_type = if file_path.ends_with(".png") {
        "image/png"
    } else if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
        "image/jpeg"
    } else if file_path.ends_with(".gif") {
        "image/gif"
    } else {
        "application/octet-stream"
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type)
        .header("cache-control", "public, max-age=31536000")
        .body(axum::body::Body::from_stream(stream))
        .unwrap();

    Ok(response)
}