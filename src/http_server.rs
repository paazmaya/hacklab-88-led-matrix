//! HTTP Server for LED Matrix Control
//!
//! This module implements a simple HTTP server that serves a web interface
//! for controlling the LED matrix display. Users can input text through
//! a web form, and the text will be displayed on the matrix.

use anyhow::{Context, Result};
use esp_idf_sys::{esp_http_server, httpd_handle_t, httpd_start, httpd_stop};
use esp_idf_sys::{
    httpd_config_t, httpd_method_t, httpd_register_uri_handler, httpd_req_t, httpd_resp_send,
    httpd_resp_send_404, httpd_resp_set_hdr, httpd_resp_set_type, httpd_uri_t, HTTPD_204,
};
use log::{debug, error, info};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::{Arc, Mutex};

use crate::led_matrix::LedMatrix;

/// HTML content for the web interface
const HTML_PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LED Matrix Controller</title>
    <style>
        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
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
            border: 1px solid rgba(255, 255, 255, 0.1);
        }
        h1 {
            color: #fff;
            text-align: center;
            margin-bottom: 10px;
            font-size: 2em;
            text-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
        }
        .subtitle {
            color: rgba(255, 255, 255, 0.7);
            text-align: center;
            margin-bottom: 30px;
            font-size: 0.9em;
        }
        .form-group {
            margin-bottom: 20px;
        }
        label {
            display: block;
            color: #fff;
            margin-bottom: 8px;
            font-weight: 500;
        }
        input[type="text"] {
            width: 100%;
            padding: 15px;
            border: 2px solid rgba(255, 255, 255, 0.2);
            border-radius: 10px;
            background: rgba(255, 255, 255, 0.1);
            color: #fff;
            font-size: 1.2em;
            transition: all 0.3s ease;
        }
        input[type="text"]:focus {
            outline: none;
            border-color: #e94560;
            background: rgba(255, 255, 255, 0.15);
        }
        input[type="text"]::placeholder {
            color: rgba(255, 255, 255, 0.5);
        }
        button {
            width: 100%;
            padding: 15px;
            background: linear-gradient(135deg, #e94560, #ff6b6b);
            border: none;
            border-radius: 10px;
            color: #fff;
            font-size: 1.1em;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
            text-transform: uppercase;
            letter-spacing: 1px;
        }
        button:hover {
            transform: translateY(-2px);
            box-shadow: 0 5px 20px rgba(233, 69, 96, 0.4);
        }
        button:active {
            transform: translateY(0);
        }
        .info {
            margin-top: 30px;
            padding: 20px;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 10px;
            border-left: 4px solid #e94560;
        }
        .info h3 {
            color: #fff;
            margin-bottom: 10px;
        }
        .info p {
            color: rgba(255, 255, 255, 0.7);
            font-size: 0.9em;
            line-height: 1.6;
        }
        .status {
            margin-top: 20px;
            padding: 15px;
            background: rgba(76, 175, 80, 0.2);
            border-radius: 10px;
            color: #4caf50;
            text-align: center;
            display: none;
        }
        .status.show {
            display: block;
            animation: fadeIn 0.3s ease;
        }
        @keyframes fadeIn {
            from { opacity: 0; transform: translateY(-10px); }
            to { opacity: 1; transform: translateY(0); }
        }
        .preview {
            margin-top: 20px;
            text-align: center;
        }
        .preview-label {
            color: rgba(255, 255, 255, 0.7);
            margin-bottom: 10px;
        }
        .preview-box {
            background: #000;
            padding: 20px;
            border-radius: 10px;
            display: inline-block;
            min-width: 200px;
        }
        .preview-text {
            color: #fff;
            font-family: monospace;
            font-size: 1.5em;
            letter-spacing: 2px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>LED Matrix</h1>
        <p class="subtitle">88x88 RGB Display Controller</p>

        <form id="textForm" onsubmit="submitText(event)">
            <div class="form-group">
                <label for="displayText">Enter text to display:</label>
                <input type="text" id="displayText" name="text"
                       placeholder="Type your message..."
                       maxlength="14" autocomplete="off">
            </div>
            <button type="submit">Display Text</button>
        </form>

        <div class="status" id="status">Text updated successfully!</div>

        <div class="preview">
            <p class="preview-label">Preview:</p>
            <div class="preview-box">
                <span class="preview-text" id="preview">&nbsp;</span>
            </div>
        </div>

        <div class="info">
            <h3>Information</h3>
            <p>
                This interface controls an 88x88 RGB LED matrix display.
                Maximum text length is 14 characters. The display uses
                16-bit PWM per color channel for smooth brightness control.
            </p>
        </div>
    </div>

    <script>
        const textInput = document.getElementById('displayText');
        const preview = document.getElementById('preview');
        const status = document.getElementById('status');

        // Live preview
        textInput.addEventListener('input', function() {
            preview.textContent = this.value || '\u00A0';
        });

        // Submit text to display
        async function submitText(event) {
            event.preventDefault();
            const text = textInput.value;

            try {
                const response = await fetch('/text?msg=' + encodeURIComponent(text));
                if (response.ok) {
                    status.classList.add('show');
                    setTimeout(() => status.classList.remove('show'), 2000);
                }
            } catch (error) {
                console.error('Error:', error);
            }
        }

        // Focus input on load
        textInput.focus();
    </script>
</body>
</html>
"#;

/// Global reference to the LED matrix
static mut LED_MATRIX: Option<Arc<Mutex<LedMatrix>>> = None;

/// Start the HTTP server
pub fn start_http_server(led_matrix: Arc<Mutex<LedMatrix>>) -> Result<()> {
    // Store global reference
    unsafe {
        LED_MATRIX = Some(led_matrix);
    }

    info!("Starting HTTP server on port 80...");

    // Create server configuration
    let mut config: httpd_config_t = unsafe { std::mem::zeroed() };
    config.server_port = 80;
    config.max_uri_handlers = 4;
    config.max_open_sockets = 4;
    config.lru_purge_enable = true;
    config.recv_wait_timeout = 5;
    config.send_wait_timeout = 5;

    // Start HTTP server
    let server: httpd_handle_t = unsafe {
        let mut handle: httpd_handle_t = ptr::null_mut();
        let result = httpd_start(&mut handle, &config);
        if result != 0 {
            anyhow::bail!("Failed to start HTTP server: error {}", result);
        }
        handle
    };

    // Register URI handlers
    register_root_handler(server)?;
    register_text_handler(server)?;
    register_clear_handler(server)?;

    info!("HTTP server started successfully!");
    Ok(())
}

/// Register the root (/) handler
fn register_root_handler(server: httpd_handle_t) -> Result<()> {
    let uri = CString::new("/").context("Invalid URI")?;
    let uri_handler: httpd_uri_t = httpd_uri_t {
        uri: uri.as_ptr(),
        method: httpd_method_t_HTTP_GET,
        handler: Some(root_handler),
        user_ctx: ptr::null_mut(),
    };

    let result = unsafe { httpd_register_uri_handler(server, &uri_handler) };
    if result != 0 {
        anyhow::bail!("Failed to register root handler");
    }
    Ok(())
}

/// Register the /text handler
fn register_text_handler(server: httpd_handle_t) -> Result<()> {
    let uri = CString::new("/text").context("Invalid URI")?;
    let uri_handler: httpd_uri_t = httpd_uri_t {
        uri: uri.as_ptr(),
        method: httpd_method_t_HTTP_GET,
        handler: Some(text_handler),
        user_ctx: ptr::null_mut(),
    };

    let result = unsafe { httpd_register_uri_handler(server, &uri_handler) };
    if result != 0 {
        anyhow::bail!("Failed to register text handler");
    }
    Ok(())
}

/// Register the /clear handler
fn register_clear_handler(server: httpd_handle_t) -> Result<()> {
    let uri = CString::new("/clear").context("Invalid URI")?;
    let uri_handler: httpd_uri_t = httpd_uri_t {
        uri: uri.as_ptr(),
        method: httpd_method_t_HTTP_GET,
        handler: Some(clear_handler),
        user_ctx: ptr::null_mut(),
    };

    let result = unsafe { httpd_register_uri_handler(server, &uri_handler) };
    if result != 0 {
        anyhow::bail!("Failed to register clear handler");
    }
    Ok(())
}

/// Root handler - serves the main HTML page
unsafe extern "C" fn root_handler(req: *mut httpd_req_t) -> i32 {
    debug!("Serving root page");

    // Set content type
    let content_type = CString::new("text/html").unwrap();
    httpd_resp_set_type(req, content_type.as_ptr());

    // Set cache control header
    let cache_control = CString::new("no-cache").unwrap();
    let cache_header = CString::new("Cache-Control").unwrap();
    httpd_resp_set_hdr(req, cache_header.as_ptr(), cache_control.as_ptr());

    // Send HTML content
    let html = CString::new(HTML_PAGE).unwrap();
    httpd_resp_send(req, html.as_ptr(), html.as_bytes().len() as i32);

    0
}

/// Text handler - updates the display text
unsafe extern "C" fn text_handler(req: *mut httpd_req_t) -> i32 {
    debug!("Text handler called");

    // Parse query string
    let mut query_buf = [0u8; 256];
    let query_len = httpd_req_get_url_query_len(req);

    if query_len > 0 && query_len < query_buf.len() as i32 {
        httpd_req_get_url_query_str(
            req,
            query_buf.as_mut_ptr() as *mut i8,
            query_buf.len() as u32,
        );

        // Find 'msg=' parameter
        let query = std::ffi::CStr::from_ptr(query_buf.as_ptr() as *const i8);
        let query_str = query.to_string_lossy();

        if let Some(msg_start) = query_str.find("msg=") {
            let msg = &query_str[msg_start + 4..];
            let decoded = url_decode(msg);

            info!("Display text: {}", decoded);

            // Update the LED matrix
            if let Some(ref matrix) = LED_MATRIX {
                if let Ok(mut m) = matrix.lock() {
                    m.display_text(&decoded);
                }
            }
        }
    }

    // Send response
    let response = CString::new("OK").unwrap();
    httpd_resp_send(req, response.as_ptr(), response.as_bytes().len() as i32);

    0
}

/// Clear handler - clears the display
unsafe extern "C" fn clear_handler(req: *mut httpd_req_t) -> i32 {
    debug!("Clear handler called");

    // Clear the LED matrix
    if let Some(ref matrix) = LED_MATRIX {
        if let Ok(mut m) = matrix.lock() {
            m.clear();
        }
    }

    // Send response
    let response = CString::new("Cleared").unwrap();
    httpd_resp_send(req, response.as_ptr(), response.as_bytes().len() as i32);

    0
}

/// URL decode a string
fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            if let (Some(h), Some(l)) = (chars.next(), chars.next()) {
                if let (Some(hv), Some(lv)) = (h.to_digit(16), l.to_digit(16)) {
                    result.push(char::from_u32(hv * 16 + lv).unwrap_or('?'));
                    continue;
                }
            }
        } else if c == '+' {
            result.push(' ');
            continue;
        }
        result.push(c);
    }

    result
}

// FFI declarations for ESP-IDF HTTP server functions
extern "C" {
    fn httpd_req_get_url_query_len(req: *mut httpd_req_t) -> i32;
    fn httpd_req_get_url_query_str(req: *mut httpd_req_t, buf: *mut i8, buf_len: u32) -> i32;
}
