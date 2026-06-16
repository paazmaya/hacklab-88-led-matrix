//! Pure HTTP request parsing for the LED matrix controller.
//!
//! Extracted from the network-aware `http_server` module so the parsing
//! logic can be unit-tested on the host without any embassy/ESP32 deps.
//! [`dispatch`] returns a [`Response`] describing what the HTTP layer
//! should send back to the client and whether to update the display text.

/// Maximum length of a decoded display message.
pub const MAX_MESSAGE_LEN: usize = 32;

/// HTML body for the controller's main page. Kept here so the parser
/// can return it from [`dispatch`] without depending on the network layer.
const HTML_PAGE: &str = include_str!("http_page.html");

/// Full 200 OK response (headers + HTML page).
const OK_HTML_RESPONSE: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n";

/// Full 404 Not Found response.
pub const NOT_FOUND_RESPONSE: &[u8] =
    b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nNot Found";

/// What the HTTP layer should send back and what (if anything) to put on
/// the display.
pub struct Response {
    /// Raw bytes to write to the socket. Already includes HTTP headers.
    pub body: &'static [u8],
    /// When `Some(text)`, the HTTP handler updates the display buffer
    /// with this text. `None` means "no change".
    pub display_text: Option<heapless::String<MAX_MESSAGE_LEN>>,
}

impl Response {
    const fn html(body: &'static [u8]) -> Self {
        Self {
            body,
            display_text: None,
        }
    }

    fn html_with_text(body: &'static [u8], text: heapless::String<MAX_MESSAGE_LEN>) -> Self {
        Self {
            body,
            display_text: Some(text),
        }
    }

    const fn not_found() -> Self {
        Self {
            body: NOT_FOUND_RESPONSE,
            display_text: None,
        }
    }
}

/// Dispatch an HTTP request and return the response the network layer
/// should send back.
///
/// The response body is `OK_HTML_RESPONSE ++ HTML_PAGE` for a controller
/// page, or `NOT_FOUND_RESPONSE` for an unknown route.
pub fn dispatch(request: &[u8]) -> Response {
    let request_str = core::str::from_utf8(request).unwrap_or("");

    if is_root_request(request_str) {
        return Response::html(OK_HTML_RESPONSE);
    }
    if is_clear_request(request_str) {
        return Response::html_with_text(OK_HTML_RESPONSE, heapless::String::new());
    }
    if is_text_update_request(request_str)
        && let Some(decoded) = extract_query_message(request_str)
    {
        return Response::html_with_text(OK_HTML_RESPONSE, decoded);
    }

    Response::not_found()
}

/// Borrow the HTML page body (used by the network layer to append it
/// after [`OK_HTML_RESPONSE`]).
pub fn html_page() -> &'static [u8] {
    HTML_PAGE.as_bytes()
}

/// True for `GET /` or `GET / HTTP/1.x` (root page).
fn is_root_request(request: &str) -> bool {
    request.starts_with("GET / ") || request.starts_with("GET / HTTP")
}

/// True for `GET /clear`.
fn is_clear_request(request: &str) -> bool {
    request.contains("GET /clear")
}

/// True for `GET /text?msg=...`.
fn is_text_update_request(request: &str) -> bool {
    request.contains("GET /text?msg=")
}

/// Extract the URL-decoded `msg=` query parameter from a request, if any.
fn extract_query_message(request: &str) -> Option<heapless::String<MAX_MESSAGE_LEN>> {
    let start = request.find("msg=")?;
    let value = &request[start + 4..];
    let end = value.find([' ', '\r', '\n']).unwrap_or(value.len());
    url_decode(&value[..end])
}

/// Percent-decode a URL-encoded string, capped at [`MAX_MESSAGE_LEN`]
/// characters. Stops at the first character that would overflow.
fn url_decode(encoded: &str) -> Option<heapless::String<MAX_MESSAGE_LEN>> {
    let mut out = heapless::String::new();
    let mut chars = encoded.chars().peekable();

    while let Some(c) = chars.next() {
        let decoded_char = match c {
            '%' => decode_percent(&mut chars)?,
            '+' => ' ',
            other => other,
        };
        if out.push(decoded_char).is_err() {
            // Buffer full — stop here. Caller sees a truncated message
            // rather than no message at all.
            break;
        }
    }

    Some(out)
}

/// Decode a `%XX` percent-escape and return the resulting character.
fn decode_percent(chars: &mut core::iter::Peekable<core::str::Chars<'_>>) -> Option<char> {
    let h = chars.next()?.to_digit(16)?;
    let l = chars.next()?.to_digit(16)?;
    char::from_u32(h * 16 + l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_request_matches() {
        assert!(is_root_request("GET / HTTP/1.1"));
        assert!(is_root_request("GET / "));
        assert!(!is_root_request("GET /text HTTP/1.1"));
    }

    #[test]
    fn clear_request_matches() {
        assert!(is_clear_request("GET /clear HTTP/1.1"));
        assert!(!is_clear_request("GET / HTTP/1.1"));
    }

    #[test]
    fn text_update_request_matches() {
        assert!(is_text_update_request("GET /text?msg=hi HTTP/1.1"));
        assert!(!is_text_update_request("GET / HTTP/1.1"));
    }

    #[test]
    fn url_decode_passes_plain_text() {
        let decoded = url_decode("hello").unwrap();
        assert_eq!(decoded.as_str(), "hello");
    }

    #[test]
    fn url_decode_handles_plus_sign() {
        let decoded = url_decode("hello+world").unwrap();
        assert_eq!(decoded.as_str(), "hello world");
    }

    #[test]
    fn url_decode_handles_percent_escapes() {
        let decoded = url_decode("hello%20world").unwrap();
        assert_eq!(decoded.as_str(), "hello world");
    }

    #[test]
    fn url_decode_handles_uppercase_hex() {
        let decoded = url_decode("%2A").unwrap();
        assert_eq!(decoded.as_str(), "*");
    }

    #[test]
    fn url_decode_stops_on_truncated_escape() {
        // '%2' is incomplete — decoder returns None.
        assert!(url_decode("%2").is_none());
    }

    #[test]
    fn url_decode_handles_mixed_input() {
        let decoded = url_decode("a%20b+c%21").unwrap();
        assert_eq!(decoded.as_str(), "a b c!");
    }

    #[test]
    fn extract_query_message_basic() {
        let msg = extract_query_message("GET /text?msg=hi HTTP/1.1").unwrap();
        assert_eq!(msg.as_str(), "hi");
    }

    #[test]
    fn extract_query_message_decoded() {
        let msg = extract_query_message("GET /text?msg=hello+world HTTP/1.1").unwrap();
        assert_eq!(msg.as_str(), "hello world");
    }

    #[test]
    fn extract_query_message_missing_returns_none() {
        assert!(extract_query_message("GET / HTTP/1.1").is_none());
    }

    #[test]
    fn dispatch_root_returns_html_no_text() {
        let resp = dispatch(b"GET / HTTP/1.1");
        assert!(resp.body.starts_with(b"HTTP/1.1 200"));
        assert!(resp.display_text.is_none());
    }

    #[test]
    fn dispatch_clear_returns_html_and_empty_text() {
        let resp = dispatch(b"GET /clear HTTP/1.1");
        assert!(resp.body.starts_with(b"HTTP/1.1 200"));
        assert_eq!(resp.display_text.unwrap().as_str(), "");
    }

    #[test]
    fn dispatch_text_update_returns_html_and_decoded_text() {
        let resp = dispatch(b"GET /text?msg=hi HTTP/1.1");
        assert!(resp.body.starts_with(b"HTTP/1.1 200"));
        assert_eq!(resp.display_text.unwrap().as_str(), "hi");
    }

    #[test]
    fn dispatch_unknown_path_returns_404() {
        let resp = dispatch(b"GET /nope HTTP/1.1");
        assert!(resp.body.starts_with(b"HTTP/1.1 404"));
        assert!(resp.display_text.is_none());
    }

    #[test]
    fn html_page_is_non_empty() {
        assert!(!html_page().is_empty());
    }
}
