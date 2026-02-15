//! HTTP Server for LED Matrix Control
//!
//! This module implements a simple HTTP server that serves a web interface
//! for controlling the LED matrix display.

use crate::DISPLAY_TEXT;
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::{Duration, Timer};
use log::{debug, info};

/// HTML content for the web interface
const HTML_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LED Matrix Controller</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
        }
        .container {
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(10px);
            border-radius: 20px;
            padding: 40px;
            max-width: 500px;
            width: 100%;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }
        h1 { color: #fff; text-align: center; margin-bottom: 10px; }
        .subtitle { color: rgba(255, 255, 255, 0.7); text-align: center; margin-bottom: 30px; }
        .form-group { margin-bottom: 20px; }
        label { display: block; color: #fff; margin-bottom: 8px; }
        input[type="text"] {
            width: 100%;
            padding: 15px;
            border: 2px solid rgba(255, 255, 255, 0.2);
            border-radius: 10px;
            background: rgba(255, 255, 255, 0.1);
            color: #fff;
            font-size: 1.2em;
        }
        input[type="text"]:focus { outline: none; border-color: #e94560; }
        input[type="text"]::placeholder { color: rgba(255, 255, 255, 0.5); }
        button {
            width: 100%;
            padding: 15px;
            background: linear-gradient(135deg, #e94560, #ff6b6b);
            border: none;
            border-radius: 10px;
            color: #fff;
            font-size: 1.1em;
            cursor: pointer;
            text-transform: uppercase;
        }
        button:hover { transform: translateY(-2px); }
        .info {
            margin-top: 30px;
            padding: 20px;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 10px;
            border-left: 4px solid #e94560;
        }
        .info h3 { color: #fff; margin-bottom: 10px; }
        .info p { color: rgba(255, 255, 255, 0.7); font-size: 0.9em; line-height: 1.6; }
    </style>
</head>
<body>
    <div class="container">
        <h1>LED Matrix</h1>
        <p class="subtitle">88x88 RGB Display Controller</p>
        <form action="/text" method="get">
            <div class="form-group">
                <label for="msg">Enter text to display:</label>
                <input type="text" id="msg" name="msg" placeholder="Type your message..." maxlength="14">
            </div>
            <button type="submit">Display Text</button>
        </form>
        <div class="info">
            <h3>Information</h3>
            <p>Controls an 88x88 RGB LED matrix. Max 14 characters. Built with Rust and esp-hal.</p>
        </div>
    </div>
</body>
</html>
"#;

/// HTTP response headers
const HTTP_OK: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n";
const HTTP_NOT_FOUND: &[u8] =
    b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nNot Found";

/// Start the HTTP server task
#[embassy_executor::task]
pub async fn http_server_task() {
    info!("HTTP server task starting...");

    // Create a TCP socket buffer
    let _rx_buffer = [0u8; 4096];
    let _tx_buffer = [0u8; 4096];

    loop {
        // Create a new TCP socket
        // Note: We need the stack reference from the wifi module
        // For now, this is a simplified version

        info!("HTTP server waiting for connection...");

        // Wait and retry
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Handle an HTTP request
fn handle_request(request: &[u8]) -> (&'static [u8], Option<heapless::String<32>>) {
    // Parse the request line
    let request_str = core::str::from_utf8(request).unwrap_or("");

    // Check for GET request
    if request_str.starts_with("GET / ") || request_str.starts_with("GET / HTTP") {
        // Serve main page
        return (HTML_PAGE.as_bytes(), None);
    }

    // Check for text update
    if request_str.contains("GET /text?msg=") {
        // Extract message from query string
        if let Some(start) = request_str.find("msg=") {
            let msg_start = start + 4;
            let remaining = &request_str[msg_start..];

            // Find end of message (space, HTTP, or end of line)
            let msg_end = remaining.find([' ', '\r', '\n']).unwrap_or(remaining.len());

            let encoded_msg = core::str::from_utf8(&remaining.as_bytes()[..msg_end]).unwrap_or("");

            // URL decode the message
            let mut decoded: heapless::String<32> = heapless::String::new();
            let mut chars = encoded_msg.chars().peekable();

            while let Some(c) = chars.next() {
                if c == '%' {
                    if let (Some(h), Some(l)) = (chars.next(), chars.next()) {
                        if let (Some(hv), Some(lv)) = (h.to_digit(16), l.to_digit(16)) {
                            if let Some(ch) = char::from_u32(hv * 16 + lv) {
                                if decoded.push(ch).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                } else if c == '+' {
                    if decoded.push(' ').is_err() {
                        break;
                    }
                } else if decoded.push(c).is_err() {
                    break;
                }
            }

            return (HTML_PAGE.as_bytes(), Some(decoded));
        }
    }

    // Check for clear
    if request_str.contains("GET /clear") {
        let empty: heapless::String<32> = heapless::String::new();
        return (HTML_PAGE.as_bytes(), Some(empty));
    }

    (HTTP_NOT_FOUND, None)
}

/// Run the HTTP server loop
pub async fn run_http_server(stack: &'static Stack<'static>) {
    let mut rx_buffer = [0u8; 2048];
    let mut tx_buffer = [0u8; 8192];

    loop {
        let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);

        // Wait for connection
        if let Err(e) = socket.accept(80).await {
            debug!("Accept error: {:?}", e);
            continue;
        }

        info!("HTTP client connected");

        // Read request
        let mut request_buf = [0u8; 512];
        let len = match socket.read(&mut request_buf).await {
            Ok(l) => l,
            Err(e) => {
                debug!("Read error: {:?}", e);
                continue;
            }
        };

        // Handle request
        let (response, new_text) = handle_request(&request_buf[..len]);

        // Update display text if provided
        if let Some(text) = new_text {
            let mut display_text = DISPLAY_TEXT.lock().await;
            *display_text = text;
            info!("Display text updated");
        }

        // Send response
        let mut response_data = [0u8; 8192];
        let header = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n";
        let header_len = header.len();
        response_data[..header_len].copy_from_slice(header);
        response_data[header_len..header_len + response.len()].copy_from_slice(response);

        if let Err(e) = socket
            .write(&response_data[..header_len + response.len()])
            .await
        {
            debug!("Write error: {:?}", e);
        }

        // Close socket
        socket.close();

        info!("HTTP request handled");
    }
}
