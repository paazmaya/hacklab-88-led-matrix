//! HTTP server task — accepts connections and delegates parsing to
//! [`crate::http_request`]. Kept thin on purpose: all request parsing,
//! URL-decoding and response shaping lives in the host-testable
//! `http_request` module.

use crate::DISPLAY_TEXT;
use embassy_net::{Stack, tcp::TcpSocket};
use esp32_led_matrix::http_request;
use log::{debug, info};

/// Run the HTTP server forever, accepting one connection at a time.
#[embassy_executor::task]
pub async fn http_server_task(stack: &'static Stack<'static>) {
    info!("HTTP server task starting on port 80");
    run_http_server(stack).await;
}

/// Server loop. Each iteration accepts a connection, reads one request,
/// writes one response, then closes the socket.
pub async fn run_http_server(stack: &'static Stack<'static>) {
    let mut rx_buffer = [0u8; 2048];
    let mut tx_buffer = [0u8; 8192];

    loop {
        let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);

        if let Err(e) = socket.accept(80).await {
            debug!("Accept error: {:?}", e);
            continue;
        }
        info!("HTTP client connected");

        let response = match read_request(&mut socket).await {
            Some(buf) => http_request::dispatch(&buf),
            None => continue,
        };

        // Take ownership of the optional display text *before* the body
        // so the partial move of `Response` doesn't trouble the borrow
        // checker on the subsequent `write_response` call.
        let body = response.body;
        let text = response.display_text;
        apply_text_update(text).await;
        write_response(&mut socket, body).await;
        socket.close();
        info!("HTTP request handled");
    }
}

/// Read one HTTP request into a fixed buffer. Returns `None` on read
/// errors so the caller can move on to the next connection.
async fn read_request(socket: &mut TcpSocket<'_>) -> Option<[u8; 512]> {
    let mut buf = [0u8; 512];
    match socket.read(&mut buf).await {
        Ok(_) => Some(buf),
        Err(e) => {
            debug!("Read error: {:?}", e);
            None
        }
    }
}

/// If the parsed response carries a new display text, update the
/// shared `DISPLAY_TEXT` global. Logs the change.
async fn apply_text_update(text: Option<heapless::String<{ http_request::MAX_MESSAGE_LEN }>>) {
    if let Some(text) = text {
        let mut display_text = DISPLAY_TEXT.lock().await;
        *display_text = text;
        info!("Display text updated");
    }
}

/// Write the response body to the socket. For HTML pages we append the
/// page body after the headers; everything else already includes its
/// own headers.
async fn write_response(socket: &mut TcpSocket<'_>, body: &'static [u8]) {
    let mut response_data = [0u8; 8192];
    let mut offset;

    if body.starts_with(b"HTTP/1.1 200") {
        response_data[..body.len()].copy_from_slice(body);
        offset = body.len();
        let page = http_request::html_page();
        response_data[offset..offset + page.len()].copy_from_slice(page);
        offset += page.len();
    } else {
        response_data[..body.len()].copy_from_slice(body);
        offset = body.len();
    }

    if let Err(e) = socket.write(&response_data[..offset]).await {
        debug!("Write error: {:?}", e);
    }
}
