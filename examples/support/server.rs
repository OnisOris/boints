//! Small static server shared by the Rust preview and browser smoke test.
use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
    time::Duration,
};

fn asset<'a>(url: &'a str, prefix: &str) -> Option<(&'a str, &'static str)> {
    let path = url.split('?').next()?.strip_prefix(prefix)?;
    let path = if path.is_empty() { "index.html" } else { path };
    // Serve only build outputs, never arbitrary paths or directory listings.
    let mime = match path {
        "index.html" => "text/html; charset=utf-8",
        "mq_js_bundle.js" => "text/javascript; charset=utf-8",
        "algorithm-playground.wasm" => "application/wasm",
        "algorithm-playground.wasm.gz" => "application/gzip",
        "LOADER.md" | "LOADER-LICENSE" | ".nojekyll" => "text/plain; charset=utf-8",
        _ => return None,
    };
    Some((path, mime))
}

fn respond(mut stream: TcpStream, root: PathBuf, prefix: &str) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut reader = BufReader::new(&stream).take(8192);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut header = String::new();
    loop {
        header.clear();
        if reader.read_line(&mut header)? == 0 {
            return Ok(());
        }
        if header == "\r\n" {
            break;
        }
    }
    let mut words = request.split_whitespace();
    let method = words.next().unwrap_or("");
    let url = words.next().unwrap_or("");
    let (status, mime, body) = if !matches!(method, "GET" | "HEAD") {
        (
            "405 Method Not Allowed",
            "text/plain",
            b"GET or HEAD required".to_vec(),
        )
    } else if let Some((path, mime)) = asset(url, prefix) {
        match fs::read(root.join(path)) {
            Ok(body) => ("200 OK", mime, body),
            Err(_) => (
                "404 Not Found",
                "text/plain",
                b"Asset not found; run ./scripts/build-web.sh".to_vec(),
            ),
        }
    } else {
        ("404 Not Found", "text/plain", b"Not found".to_vec())
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    )?;
    if method != "HEAD" {
        stream.write_all(&body)?;
    }
    Ok(())
}

pub fn serve(listener: TcpListener, root: PathBuf, prefix: &str) -> io::Result<()> {
    let root = root.canonicalize()?;
    for stream in listener.incoming() {
        let stream = stream?;
        let root = root.clone();
        let prefix = prefix.to_owned();
        // Browsers can preconnect idle sockets; one must not block other requests.
        thread::spawn(move || {
            let _ = respond(stream, root, &prefix);
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::asset;
    #[test]
    fn restricts_assets_and_preserves_project_prefix() {
        assert_eq!(
            asset("/repository/", "/repository/"),
            Some(("index.html", "text/html; charset=utf-8"))
        );
        assert_eq!(
            asset("/repository/algorithm-playground.wasm?v=1", "/repository/"),
            Some(("algorithm-playground.wasm", "application/wasm"))
        );
        for path in [
            "/Cargo.toml",
            "/repository/../Cargo.toml",
            "/repository/%2e%2e/Cargo.toml",
            "/repository-other/index.html",
        ] {
            assert!(asset(path, "/repository/").is_none());
        }
    }
}
