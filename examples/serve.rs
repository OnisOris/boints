//! Local web preview with only Rust's standard library.
#[path = "support/server.rs"]
mod server;

use std::{error::Error, io, net::TcpListener, path::PathBuf};
fn main() -> Result<(), Box<dyn Error>> {
    let requested = std::env::args().nth(1);
    let address = requested.as_deref().unwrap_or("127.0.0.1:8000");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dist");
    if !root.join("index.html").is_file() {
        return Err("dist/index.html missing; run ./scripts/build-web.sh first".into());
    }
    let listener = match TcpListener::bind(address) {
        Ok(listener) => listener,
        Err(error) if requested.is_none() && error.kind() == io::ErrorKind::AddrInUse => {
            eprintln!("Port 8000 is busy; selecting a free local port.");
            TcpListener::bind("127.0.0.1:0")?
        }
        Err(error) => {
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "Cannot listen on {address}: {error}. Try another port, e.g. 127.0.0.1:8001"
                ),
            )
            .into());
        }
    };
    println!(
        "Serving dist at http://{}/ — Ctrl+C to stop",
        listener.local_addr()?
    );
    server::serve(listener, root, "/")?;
    Ok(())
}
