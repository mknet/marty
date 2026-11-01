//! Tiny HTTP front-end that execs `./cgi-bin/<name>` as CGI scripts, with extra path segments as `PATH_INFO`.
//! **For local example / test use in the Marty repo only** — not a supported production server.
//! Lives in `examples/_http-cgi-server` (underscore prefix) next to runnable example crates.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("(marty examples) _http-cgi-server — http://127.0.0.1:8080");
    println!("Place CGI binaries under ./cgi-bin/ (see workspace examples).");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    handle_connection(stream);
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

/// Method, path, query string, lowercased headers, body.
type HttpRequest = (String, String, String, HashMap<String, String>, Vec<u8>);

fn read_http_request(mut stream: &TcpStream) -> Option<HttpRequest> {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];

    loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if header_end(&buf).is_some() {
            break;
        }
        if buf.len() > 1 << 20 {
            return None;
        }
    }

    let header_len = header_end(&buf)?;
    let header_bytes = &buf[..header_len];
    let mut body = buf[header_len..].to_vec();

    let request = String::from_utf8_lossy(header_bytes);
    let mut lines = request.lines();
    let first_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 3 {
        return None;
    }

    let method = parts[0].to_string();
    let (path, query_string) = split_request_target(parts[1]);

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(": ") {
            headers.insert(key.to_lowercase(), value.to_string());
        }
    }

    let content_length: usize = headers
        .get("content-length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    while body.len() < content_length {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);

    Some((method, path, query_string, headers, body))
}

/// Split an HTTP request-target into path and query (without the `?`).
fn split_request_target(target: &str) -> (String, String) {
    match target.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (target.to_string(), String::new()),
    }
}

fn header_end(buf: &[u8]) -> Option<usize> {
    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        return Some(i + 4);
    }
    buf.windows(2).position(|w| w == b"\n\n").map(|i| i + 2)
}

fn handle_connection(stream: TcpStream) {
    let Some((method, path, query_string, headers, body)) = read_http_request(&stream) else {
        send_error(stream, "400 Bad Request");
        return;
    };

    if path.starts_with("/cgi-bin/") {
        handle_cgi_request(stream, &method, &path, &query_string, headers, body);
    } else {
        send_error(stream, "404 Not Found");
    }
}

fn handle_cgi_request(
    stream: TcpStream,
    method: &str,
    path: &str,
    query_string: &str,
    headers: HashMap<String, String>,
    request_body: Vec<u8>,
) {
    // First path segment under /cgi-bin/ is the executable; the rest is PATH_INFO (RFC 3875).
    let after_prefix = path.strip_prefix("/cgi-bin/").unwrap_or("");
    let mut segments = after_prefix
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if segments.is_empty() {
        send_error(stream, "404 Not Found");
        return;
    }

    let exe_name = segments.remove(0);
    let path_info = if segments.is_empty() {
        String::new()
    } else {
        format!("/{}", segments.join("/"))
    };

    let script_path = format!("./cgi-bin/{}", exe_name);
    let script_name_uri = format!("/cgi-bin/{}", exe_name);

    let mut absolute_script_path = std::env::current_dir().unwrap();
    absolute_script_path.push(&script_path);
    let absolute_script_path = match absolute_script_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            send_error(stream, "404 Not Found");
            return;
        }
    };

    // Check if script exists
    if !absolute_script_path.is_file() {
        send_error(stream, "404 Not Found");
        return;
    }

    // Set up CGI environment variables
    let mut env_vars = HashMap::new();
    let content_length = headers
        .get("content-length")
        .unwrap_or(&"0".to_string())
        .clone();
    let content_type = headers
        .get("content-type")
        .unwrap_or(&"text/plain".to_string())
        .clone();

    let request_uri = if query_string.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{query_string}")
    };

    env_vars.insert("REQUEST_METHOD".to_string(), method.to_string());
    env_vars.insert("CONTENT_LENGTH".to_string(), content_length);
    env_vars.insert("REQUEST_URI".to_string(), request_uri);
    env_vars.insert("QUERY_STRING".to_string(), query_string.to_string());
    env_vars.insert("CONTENT_TYPE".to_string(), content_type);
    env_vars.insert("SERVER_PROTOCOL".to_string(), "HTTP/1.1".to_string());
    env_vars.insert("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string());
    env_vars.insert(
        "SERVER_SOFTWARE".to_string(),
        "marty-_http-cgi-server/1.0".to_string(),
    );
    env_vars.insert("REMOTE_ADDR".to_string(), "127.0.0.1".to_string());
    env_vars.insert("REMOTE_PORT".to_string(), "12345".to_string());
    env_vars.insert("SERVER_ADDR".to_string(), "127.0.0.1".to_string());
    env_vars.insert("SERVER_PORT".to_string(), "8080".to_string());
    env_vars.insert("SERVER_NAME".to_string(), "localhost".to_string());
    env_vars.insert("DOCUMENT_ROOT".to_string(), ".".to_string());
    env_vars.insert("SCRIPT_NAME".to_string(), script_name_uri);
    env_vars.insert("PATH_INFO".to_string(), path_info);
    env_vars.insert("PATH_TRANSLATED".to_string(), script_path.clone());

    // Add HTTP headers as environment variables
    for (key, value) in headers {
        let env_key = format!("HTTP_{}", key.to_uppercase().replace("-", "_"));
        env_vars.insert(env_key, value);
    }

    // Execute the CGI script
    let mut command = Command::new(&absolute_script_path);

    // Set environment variables
    for (key, value) in env_vars {
        command.env(key, value);
    }

    let output = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    match output {
        Ok(mut child) => {
            // Send any input to the CGI script
            if let Some(mut stdin) = child.stdin.take()
                && !request_body.is_empty()
            {
                stdin.write_all(&request_body).unwrap();
            }

            // Read the output
            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();

            let mut output = Vec::new();
            let mut error_output = Vec::new();

            stdout.read_to_end(&mut output).unwrap();
            stderr.read_to_end(&mut error_output).unwrap();

            if !error_output.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&error_output));
            }

            // Wait for the process to complete
            let status = child.wait().unwrap();

            // Parse and send the CGI response
            let response = String::from_utf8_lossy(&output);
            send_cgi_response(stream, &response);

            if !status.success() {
                eprintln!("CGI script exited with status: {}", status);
            }
        }
        Err(e) => {
            eprintln!("Failed to execute CGI script: {}", e);
            send_error(stream, "500 Internal Server Error");
        }
    }
}

fn send_cgi_response(mut stream: TcpStream, cgi_output: &str) {
    // CGI "NPH-style" document: header block (incl. `Status:`), blank line, body.
    // Marty uses `\n` line endings; normalise for parsing then emit `\r\n` on the wire.
    let text = cgi_output.replace('\r', "");
    let mut parts = text.splitn(2, "\n\n");
    let headers = parts.next();
    let body = parts.next();

    if let (Some(headers), Some(body)) = (headers, body) {
        let mut status_line = "HTTP/1.1 200 OK\r\n".to_string();
        let mut out_headers = String::new();
        for line in headers.lines() {
            let key = line.split(':').next().unwrap_or("").trim();
            if key.eq_ignore_ascii_case("status") {
                let value = line.split_once(':').map_or("200", |(_, v)| v).trim();
                status_line = format!("HTTP/1.1 {}\r\n", value);
            } else if !line.is_empty() {
                out_headers.push_str(line);
                out_headers.push_str("\r\n");
            }
        }

        stream.write_all(status_line.as_bytes()).unwrap();
        stream.write_all(out_headers.as_bytes()).unwrap();
        stream.write_all(b"\r\n").unwrap();
        stream.write_all(body.as_bytes()).unwrap();
    } else {
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n")
            .unwrap();
        stream.write_all(cgi_output.as_bytes()).unwrap();
    }
}

fn send_error(mut stream: TcpStream, status: &str) {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        status,
        status.len(),
        status
    );
    stream.write_all(response.as_bytes()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::split_request_target;

    #[test]
    fn split_request_target_without_query() {
        assert_eq!(
            split_request_target("/cgi-bin/marty-02-routing/hello/Doc"),
            (
                "/cgi-bin/marty-02-routing/hello/Doc".to_string(),
                String::new()
            )
        );
    }

    #[test]
    fn split_request_target_with_query() {
        assert_eq!(
            split_request_target("/cgi-bin/marty-02-routing/hello/Doc?x=1&y=2"),
            (
                "/cgi-bin/marty-02-routing/hello/Doc".to_string(),
                "x=1&y=2".to_string()
            )
        );
    }
}
