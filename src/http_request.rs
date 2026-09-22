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

/// A structured command describing what to render to the LED matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayCommand {
    /// Text to render.
    pub text: heapless::String<MAX_MESSAGE_LEN>,
    /// Optional X start coordinate (if `None`, centers horizontally / starts at margin).
    pub x: Option<i32>,
    /// Optional Y start coordinate (if `None`, centers vertically).
    pub y: Option<i32>,
    /// 16-bit RGB color channels `[R, G, B]`.
    pub color: [u16; 3],
    /// Whether to clear the display buffer before drawing.
    pub clear: bool,
}

impl Default for DisplayCommand {
    fn default() -> Self {
        Self {
            text: heapless::String::new(),
            x: None,
            y: None,
            color: [0xFFFF, 0xFFFF, 0xFFFF],
            clear: true,
        }
    }
}

/// What the HTTP layer should send back and what (if anything) to put on
/// the display.
pub struct Response {
    /// Raw bytes to write to the socket. Already includes HTTP headers.
    pub body: &'static [u8],
    /// When `Some(text)`, the HTTP handler updates the display buffer
    /// with this text. `None` means "no change". Retained for backwards compatibility.
    pub display_text: Option<heapless::String<MAX_MESSAGE_LEN>>,
    /// Full structured display command (position, hex color, clear flag).
    pub command: Option<DisplayCommand>,
}

impl Response {
    const fn html(body: &'static [u8]) -> Self {
        Self {
            body,
            display_text: None,
            command: None,
        }
    }

    fn html_with_command(body: &'static [u8], command: DisplayCommand) -> Self {
        Self {
            body,
            display_text: Some(command.text.clone()),
            command: Some(command),
        }
    }

    const fn not_found() -> Self {
        Self {
            body: NOT_FOUND_RESPONSE,
            display_text: None,
            command: None,
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
        return Response::html_with_command(
            OK_HTML_RESPONSE,
            DisplayCommand {
                text: heapless::String::new(),
                x: None,
                y: None,
                color: [0xFFFF, 0xFFFF, 0xFFFF],
                clear: true,
            },
        );
    }
    if is_text_update_request(request_str) {
        if let Some(cmd) = extract_display_command(request_str) {
            return Response::html_with_command(OK_HTML_RESPONSE, cmd);
        }
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

/// True for `GET /text?` or `GET /text `.
fn is_text_update_request(request: &str) -> bool {
    request.contains("GET /text?") || request.contains("GET /text ")
}

/// Parse a 6-digit hex color into 16-bit PWM RGB values `[R, G, B]`.
///
/// Accepts `#RRGGBB`, `%23RRGGBB`, `0xRRGGBB`, or `RRGGBB` (case-insensitive).
/// Each 8-bit channel is scaled to 16-bit PWM ($c \times 257$).
pub fn parse_hex_color(val: &str) -> Option<[u16; 3]> {
    let s = val
        .strip_prefix("%23")
        .or_else(|| val.strip_prefix('%'))
        .unwrap_or(val);
    let s = s.strip_prefix('#').unwrap_or(s);
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);

    if s.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;

    Some([r as u16 * 257, g as u16 * 257, b as u16 * 257])
}

/// Extract a structured [`DisplayCommand`] from an HTTP request.
pub fn extract_display_command(request: &str) -> Option<DisplayCommand> {
    let start = request.find("GET /text")?;
    let after_path = &request[start + "GET /text".len()..];
    if !after_path.starts_with('?') {
        return Some(DisplayCommand::default());
    }
    let query = &after_path[1..];
    let query_end = query.find([' ', '\r', '\n']).unwrap_or(query.len());
    let query = &query[..query_end];

    let mut text = heapless::String::new();
    let mut x = None;
    let mut y = None;
    let mut color = [0xFFFF, 0xFFFF, 0xFFFF];
    let mut clear = None;

    for param in query.split('&') {
        if param.is_empty() {
            continue;
        }
        let mut parts = param.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");

        match key {
            "msg" => {
                if let Some(decoded) = url_decode(val) {
                    text = decoded;
                }
            }
            "x" => {
                if let Ok(val_i32) = val.parse::<i32>() {
                    x = Some(val_i32);
                }
            }
            "y" => {
                if let Ok(val_i32) = val.parse::<i32>() {
                    y = Some(val_i32);
                }
            }
            "color" => {
                if let Some(parsed_color) = parse_hex_color(val) {
                    color = parsed_color;
                }
            }
            "clear" => {
                clear = Some(val == "1" || val.eq_ignore_ascii_case("true"));
            }
            _ => {}
        }
    }

    Some(DisplayCommand {
        text,
        x,
        y,
        color,
        clear: clear.unwrap_or(true),
    })
}

/// Extract the URL-decoded `msg=` query parameter from a request, if any.
pub fn extract_query_message(request: &str) -> Option<heapless::String<MAX_MESSAGE_LEN>> {
    let start = request.find("msg=")?;
    let value = &request[start + 4..];
    let end = value.find(['&', ' ', '\r', '\n']).unwrap_or(value.len());
    url_decode(&value[..end])
}

/// Percent-decode a URL-encoded string, capped at [`MAX_MESSAGE_LEN`]
/// characters. Stops at the first character that would overflow.
/// Converts escaped line breaks (`\n` or `\N`) into actual newline characters (`\n`).
fn url_decode(encoded: &str) -> Option<heapless::String<MAX_MESSAGE_LEN>> {
    let mut out = heapless::String::new();
    let mut chars = encoded.chars().peekable();

    while let Some(c) = chars.next() {
        let mut decoded_char = match c {
            '%' => decode_percent(&mut chars)?,
            '+' => ' ',
            other => other,
        };
        // Unescape literal \n or \N from web forms or query strings to real newline
        if decoded_char == '\\' {
            if let Some(&next_c) = chars.peek() {
                if next_c == 'n' || next_c == 'N' {
                    chars.next();
                    decoded_char = '\n';
                }
            }
        }
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

    #[test]
    fn parse_hex_color_formats() {
        // Full red
        assert_eq!(parse_hex_color("#FF0000"), Some([0xFFFF, 0x0000, 0x0000]));
        // URL-encoded #
        assert_eq!(parse_hex_color("%2300FF00"), Some([0x0000, 0xFFFF, 0x0000]));
        // Raw 6-char hex
        assert_eq!(parse_hex_color("0000FF"), Some([0x0000, 0x0000, 0xFFFF]));
        // Mixed case and 0x prefix
        assert_eq!(
            parse_hex_color("0x12ab34"),
            Some([0x12 * 257, 0xab * 257, 0x34 * 257])
        );
        // Invalid lengths or characters
        assert_eq!(parse_hex_color("FFF"), None);
        assert_eq!(parse_hex_color("ZZZZZZ"), None);
    }

    #[test]
    fn extract_display_command_parses_all_fields() {
        let req = "GET /text?msg=test&x=10&y=20&color=FF8000&clear=0 HTTP/1.1";
        let cmd = extract_display_command(req).unwrap();
        assert_eq!(cmd.text.as_str(), "test");
        assert_eq!(cmd.x, Some(10));
        assert_eq!(cmd.y, Some(20));
        assert_eq!(cmd.color, [0xFF * 257, 0x80 * 257, 0x00 * 257]);
        assert_eq!(cmd.clear, false);
    }

    #[test]
    fn extract_display_command_defaults_when_omitted() {
        let req = "GET /text?msg=hello HTTP/1.1";
        let cmd = extract_display_command(req).unwrap();
        assert_eq!(cmd.text.as_str(), "hello");
        assert_eq!(cmd.x, None);
        assert_eq!(cmd.y, None);
        assert_eq!(cmd.color, [0xFFFF, 0xFFFF, 0xFFFF]);
        assert_eq!(cmd.clear, true);
    }

    #[test]
    fn url_decode_unescapes_literal_newline() {
        let decoded = url_decode(r"line1\nline2").unwrap();
        assert_eq!(decoded.as_str(), "line1\nline2");

        let decoded_upper = url_decode(r"line1\Nline2").unwrap();
        assert_eq!(decoded_upper.as_str(), "line1\nline2");
    }

    #[test]
    fn url_decode_unescapes_percent_encoded_backslash_newline() {
        let decoded = url_decode("line1%5Cnline2").unwrap();
        assert_eq!(decoded.as_str(), "line1\nline2");
    }

    #[test]
    fn url_decode_unescapes_percent_0a_newline() {
        let decoded = url_decode("line1%0Aline2").unwrap();
        assert_eq!(decoded.as_str(), "line1\nline2");
    }

    #[test]
    fn url_decode_preserves_single_backslash_if_not_n() {
        let decoded = url_decode(r"A\tB\rC").unwrap();
        assert_eq!(decoded.as_str(), r"A\tB\rC");
    }

    #[test]
    fn extract_display_command_negative_coords() {
        let req = "GET /text?msg=hi&x=-12&y=-34 HTTP/1.1";
        let cmd = extract_display_command(req).unwrap();
        assert_eq!(cmd.x, Some(-12));
        assert_eq!(cmd.y, Some(-34));
    }

    #[test]
    fn extract_display_command_unknown_query_keys_ignored() {
        let req = "GET /text?msg=ok&foo=bar&clear=true&unknown=123 HTTP/1.1";
        let cmd = extract_display_command(req).unwrap();
        assert_eq!(cmd.text.as_str(), "ok");
        assert_eq!(cmd.clear, true);
    }

    #[test]
    fn extract_display_command_boolean_clear_variations() {
        let cmd1 = extract_display_command("GET /text?clear=1 HTTP/1.1").unwrap();
        assert!(cmd1.clear);

        let cmd2 = extract_display_command("GET /text?clear=true HTTP/1.1").unwrap();
        assert!(cmd2.clear);

        let cmd3 = extract_display_command("GET /text?clear=TRUE HTTP/1.1").unwrap();
        assert!(cmd3.clear);

        let cmd4 = extract_display_command("GET /text?clear=0 HTTP/1.1").unwrap();
        assert!(!cmd4.clear);

        let cmd5 = extract_display_command("GET /text?clear=false HTTP/1.1").unwrap();
        assert!(!cmd5.clear);
    }

    #[test]
    fn display_command_default_values() {
        let def = DisplayCommand::default();
        assert_eq!(def.text.as_str(), "");
        assert_eq!(def.x, None);
        assert_eq!(def.y, None);
        assert_eq!(def.color, [0xFFFF, 0xFFFF, 0xFFFF]);
        assert!(def.clear);
    }

    #[test]
    fn extract_display_command_without_query_returns_default() {
        let cmd = extract_display_command("GET /text HTTP/1.1").unwrap();
        assert_eq!(cmd, DisplayCommand::default());

        let cmd_no_http = extract_display_command("GET /text").unwrap();
        assert_eq!(cmd_no_http, DisplayCommand::default());
    }

    #[test]
    fn extract_display_command_empty_query_params_ignored() {
        let cmd = extract_display_command("GET /text?&msg=hello&&clear=0& HTTP/1.1").unwrap();
        assert_eq!(cmd.text.as_str(), "hello");
        assert!(!cmd.clear);
    }
}
