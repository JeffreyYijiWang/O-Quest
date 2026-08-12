use axum::body::{Body, HttpBody, to_bytes};
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;

const MAX_COMPRESSIBLE_BODY: usize = 2 * 1024 * 1024;
const MIN_COMPRESSIBLE_BODY: usize = 1024;

pub async fn gzip_responses(request: Request, next: Next) -> Response {
    let accepts_gzip = request
        .headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(accepts_gzip);

    let response = next.run(request).await;
    let compressible_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json") || value.starts_with("text/"));
    let bounded_body = response
        .body()
        .size_hint()
        .upper()
        .is_some_and(|size| size <= MAX_COMPRESSIBLE_BODY as u64);
    if !accepts_gzip
        || !compressible_type
        || !bounded_body
        || response.headers().contains_key(header::CONTENT_ENCODING)
    {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let bytes = match to_bytes(body, MAX_COMPRESSIBLE_BODY).await {
        Ok(bytes) => bytes,
        Err(_) => {
            parts.status = StatusCode::INTERNAL_SERVER_ERROR;
            return Response::from_parts(parts, Body::from("response compression failed"));
        }
    };
    if bytes.len() < MIN_COMPRESSIBLE_BODY {
        return Response::from_parts(parts, Body::from(bytes));
    }

    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    if encoder.write_all(&bytes).is_err() {
        return Response::from_parts(parts, Body::from(bytes));
    }
    let compressed = match encoder.finish() {
        Ok(compressed) => compressed,
        Err(_) => return Response::from_parts(parts, Body::from(bytes)),
    };

    parts
        .headers
        .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    parts
        .headers
        .append(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(compressed))
}

fn accepts_gzip(value: &str) -> bool {
    value.split(',').any(|item| {
        let mut parts = item.trim().split(';');
        let Some(coding) = parts.next() else {
            return false;
        };
        if !coding.eq_ignore_ascii_case("gzip") {
            return false;
        }
        parts.all(|parameter| {
            let Some((name, value)) = parameter.trim().split_once('=') else {
                return true;
            };
            !name.eq_ignore_ascii_case("q")
                || value.parse::<f32>().is_ok_and(|quality| quality > 0.0)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::accepts_gzip;

    #[test]
    fn honors_gzip_quality() {
        assert!(accepts_gzip("br, gzip;q=0.8"));
        assert!(accepts_gzip("GZIP"));
        assert!(!accepts_gzip("br, gzip;q=0"));
        assert!(!accepts_gzip("identity"));
    }
}
