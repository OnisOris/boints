//! Native Rust Chromium/CDP smoke test; no Python, Node.js, or running server needed.
#[path = "support/server.rs"]
mod server;

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn wait(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

// Kill Chromium before its temporary profile directory is removed, even on failure.
struct Browser {
    process: Child,
    profile: tempfile::TempDir,
}
impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
impl Browser {
    fn start(out: &Path) -> Result<Self> {
        let profile = tempfile::tempdir()?;
        let log = File::create(out.join("chromium.log"))?;
        let candidates = std::env::var("CHROMIUM")
            .map(|p| vec![p])
            .unwrap_or_else(|_| vec!["chromium".into(), "google-chrome".into()]);
        for executable in candidates {
            match Command::new(executable)
                .args([
                    "--headless",
                    "--no-sandbox",
                    "--disable-dev-shm-usage",
                    "--use-gl=angle",
                    "--use-angle=swiftshader",
                    "--enable-unsafe-swiftshader",
                    "--remote-debugging-port=0",
                    "--window-size=1100,760",
                ])
                .arg(format!("--user-data-dir={}", profile.path().display()))
                .arg("about:blank")
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log.try_clone()?))
                .spawn()
            {
                Ok(process) => return Ok(Self { process, profile }),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err("Install Chromium or set CHROMIUM to its executable path".into())
    }
    fn endpoint(&mut self) -> Result<(String, String)> {
        let file = self.profile.path().join("DevToolsActivePort");
        let start = Instant::now();
        while !file.exists() {
            if start.elapsed() > Duration::from_secs(10) || self.process.try_wait()?.is_some() {
                return Err("Chromium did not start; inspect tmp/smoke/chromium.log".into());
            }
            wait(50);
        }
        let port = fs::read_to_string(file)?
            .lines()
            .next()
            .ok_or("Missing DevTools port")?
            .to_owned();
        let address = format!("127.0.0.1:{port}");
        let mut stream = TcpStream::connect(&address)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        write!(
            stream,
            "GET /json HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )?;
        // DevTools can keep the TCP connection open despite Connection: close.
        // Read the declared HTTP body length rather than waiting for EOF.
        let mut reader = BufReader::new(stream).take(1024 * 1024);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if !line.starts_with("HTTP/1.1 200 ") {
            return Err(format!("DevTools HTTP: {line}").into());
        }
        let mut length = None;
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                return Err("Truncated DevTools headers".into());
            }
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = Some(value.trim().parse::<usize>()?);
            }
        }
        let length = length
            .filter(|n| *n < 1024 * 1024)
            .ok_or("Missing or oversized DevTools body length")?;
        let mut body = vec![0; length];
        reader.read_exact(&mut body)?;
        let pages: Value = serde_json::from_slice(&body)?;
        let url = pages
            .as_array()
            .ok_or("No DevTools pages")?
            .iter()
            .find(|page| page["type"] == "page")
            .and_then(|p| p["webSocketDebuggerUrl"].as_str())
            .ok_or("No DevTools page endpoint")?
            .to_owned();
        Ok((address, url))
    }
}

struct Cdp {
    socket: WebSocket<TcpStream>,
    number: u64,
    events: Vec<Value>,
}
impl Cdp {
    fn connect(address: &str, url: &str) -> Result<Self> {
        let stream = TcpStream::connect(address)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let (socket, _) = tungstenite::client(url, stream)?;
        Ok(Self {
            socket,
            number: 0,
            events: Vec::new(),
        })
    }
    fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        self.number += 1;
        self.socket.send(Message::Text(
            json!({"id":self.number,"method":method,"params":params})
                .to_string()
                .into(),
        ))?;
        loop {
            match self.socket.read()? {
                Message::Text(text) => {
                    let reply: Value = serde_json::from_str(&text)?;
                    if reply["id"] == self.number {
                        if !reply["error"].is_null() {
                            return Err(format!("CDP {method}: {}", reply["error"]).into());
                        }
                        return Ok(reply["result"].clone());
                    }
                    self.events.push(reply);
                }
                Message::Ping(data) => self.socket.send(Message::Pong(data))?,
                Message::Close(_) => return Err("CDP connection closed".into()),
                _ => {}
            }
        }
    }
    fn evaluate(&mut self, expression: &str, promise: bool) -> Result<Value> {
        let reply = self.call(
            "Runtime.evaluate",
            json!({"expression":expression,"returnByValue":true,"awaitPromise":promise}),
        )?;
        if !reply["exceptionDetails"].is_null() {
            return Err(format!("JavaScript: {reply}").into());
        }
        Ok(reply["result"]["value"].clone())
    }
    fn click(&mut self, x: u32, y: u32) -> Result<()> {
        for kind in ["mousePressed", "mouseReleased"] {
            self.call(
                "Input.dispatchMouseEvent",
                json!({"type":kind,"x":x,"y":y,"button":"left","clickCount":1}),
            )?;
        }
        wait(200);
        Ok(())
    }
    fn touch(&mut self, kind: &str, points: &[(u32, u32, u32)]) -> Result<()> {
        let points: Vec<_> = points
            .iter()
            .map(|(id, x, y)| json!({"id":id,"x":x,"y":y}))
            .collect();
        self.call(
            "Input.dispatchTouchEvent",
            json!({"type":kind,"touchPoints":points}),
        )?;
        wait(150);
        Ok(())
    }
    fn screenshot(
        &mut self,
        out: &Path,
        name: Option<&str>,
        mobile_scene: Option<bool>,
    ) -> Result<Vec<u8>> {
        let mut params = json!({"format":"png"});
        if let Some(mobile) = mobile_scene {
            let (width, height, bottom) = if mobile {
                (390, 844, 160)
            } else {
                (1100, 760, 112)
            };
            params["clip"] =
                json!({"x":0,"y":84,"width":width,"height":height-84-bottom,"scale":1});
        }
        let reply = self.call("Page.captureScreenshot", params)?;
        let data = STANDARD.decode(reply["data"].as_str().ok_or("Missing screenshot")?)?;
        if let Some(name) = name {
            fs::write(out.join(name), &data)?;
        }
        Ok(data)
    }
    fn viewport(&mut self, width: u32, height: u32, mobile: bool) -> Result<()> {
        self.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":height,"deviceScaleFactor":1,"mobile":mobile}),
        )?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if !root.join("dist/index.html").is_file() {
        return Err("Run ./scripts/build-web.sh first".into());
    }
    let out = root.join("tmp/smoke");
    fs::create_dir_all(&out)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/repository/", listener.local_addr()?);
    thread::spawn(move || {
        let _ = server::serve(listener, root.join("dist"), "/repository/");
    });
    let mut browser = Browser::start(&out)?;
    let (address, endpoint) = browser.endpoint()?;
    let mut cdp = Cdp::connect(&address, &endpoint)?;
    for domain in ["Runtime.enable", "Network.enable", "Page.enable"] {
        cdp.call(domain, json!({}))?;
    }
    cdp.viewport(1100, 760, false)?;
    cdp.call("Page.navigate", json!({"url":url}))?;
    let start = Instant::now();
    loop {
        if cdp.evaluate("typeof wasm_exports !== 'undefined' && !!wasm_exports && document.getElementById('status').style.display === 'none'",false)? == true { break; }
        if cdp.evaluate("typeof appFailed !== 'undefined' && appFailed", false)? == true {
            return Err(format!(
                "WASM startup: {}",
                cdp.evaluate("document.getElementById('status').textContent", false)?
            )
            .into());
        }
        if start.elapsed() > Duration::from_secs(30) {
            return Err("WASM did not start within 30 seconds".into());
        }
        wait(100);
    }
    wait(1000);
    assert_eq!(
        cdp.evaluate("!!gl && !gl.isContextLost()", false)?,
        true,
        "No live WebGL context"
    );
    cdp.screenshot(&out, Some("desktop.png"), None)?;
    let measure_fps = "new Promise(resolve => { const start = performance.now(); let frames=0; function sample(now) { frames++; if(now-start>=2000) resolve(+(frames*1000/(now-start)).toFixed(1)); else requestAnimationFrame(sample); } requestAnimationFrame(sample); })";
    let fps100 = cdp.evaluate(measure_fps, true)?;
    for _ in 0..4 {
        cdp.click(750, 720)?;
    } // +25 agents per click.
    let fps200 = cdp.evaluate(measure_fps, true)?;
    cdp.screenshot(&out, Some("desktop-200.png"), None)?;
    for _ in 0..4 {
        cdp.click(610, 720)?;
    }
    cdp.click(60, 720)?; // Pause.
    let a = cdp.screenshot(&out, None, Some(false))?;
    wait(300);
    assert_eq!(
        a,
        cdp.screenshot(&out, None, Some(false))?,
        "Pause did not freeze scene"
    );
    cdp.click(200, 720)?;
    assert_ne!(
        a,
        cdp.screenshot(&out, None, Some(false))?,
        "Step did not move scene"
    );
    let before_reset = cdp.screenshot(&out, None, Some(false))?;
    cdp.click(335, 720)?;
    let a = cdp.screenshot(&out, None, Some(false))?;
    assert_ne!(before_reset, a, "Reset did not recreate scene");
    cdp.call(
        "Input.dispatchMouseEvent",
        json!({"type":"mousePressed","x":600,"y":300,"button":"left","buttons":1}),
    )?;
    wait(150);
    cdp.call(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":730,"y":340,"button":"left","buttons":1}),
    )?;
    wait(200);
    cdp.call(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseReleased","x":730,"y":340,"button":"left"}),
    )?;
    wait(200);
    assert_ne!(
        a,
        cdp.screenshot(&out, None, Some(false))?,
        "Mouse orbit failed"
    );
    cdp.click(890, 720)?;
    assert_eq!(
        a,
        cdp.screenshot(&out, None, Some(false))?,
        "Camera reset failed"
    );
    cdp.click(400, 670)?;
    assert_ne!(
        a,
        cdp.screenshot(&out, None, Some(false))?,
        "Algorithm switch failed"
    );
    cdp.screenshot(&out, Some("desktop-other-algorithm.png"), None)?;
    cdp.viewport(390, 844, true)?;
    cdp.call(
        "Emulation.setTouchEmulationEnabled",
        json!({"enabled":true,"maxTouchPoints":5}),
    )?;
    wait(500);
    let a = cdp.screenshot(&out, None, Some(true))?;
    cdp.touch("touchStart", &[(1, 190, 340)])?;
    cdp.touch("touchMove", &[(1, 270, 400)])?;
    cdp.touch("touchEnd", &[])?;
    assert_ne!(
        a,
        cdp.screenshot(&out, None, Some(true))?,
        "Touch orbit failed"
    );
    let a = cdp.screenshot(&out, None, Some(true))?;
    cdp.touch("touchStart", &[(1, 120, 350), (2, 270, 350)])?;
    cdp.touch("touchMove", &[(1, 70, 350), (2, 320, 350)])?;
    cdp.touch("touchEnd", &[])?;
    assert_ne!(a, cdp.screenshot(&out, None, Some(true))?, "Pinch failed");
    let a = cdp.screenshot(&out, None, Some(true))?;
    // A gesture starting on UI remains captured when it crosses into the scene.
    cdp.touch("touchStart", &[(1, 190, 40)])?;
    cdp.touch("touchMove", &[(1, 280, 400)])?;
    cdp.touch("touchEnd", &[])?;
    assert_eq!(
        a,
        cdp.screenshot(&out, None, Some(true))?,
        "UI touch rotated camera"
    );
    cdp.screenshot(&out, Some("mobile-emulation.png"), None)?;
    cdp.viewport(844, 390, true)?;
    wait(500);
    cdp.screenshot(&out, Some("landscape-emulation.png"), None)?;
    let mut resources = Vec::new();
    for event in &cdp.events {
        assert_ne!(
            event["method"], "Runtime.exceptionThrown",
            "Browser error: {event}"
        );
        assert!(
            !(event["method"] == "Runtime.consoleAPICalled" && event["params"]["type"] == "error"),
            "Console error: {event}"
        );
        if event["method"] == "Network.responseReceived" {
            let response = &event["params"]["response"];
            assert!(
                response["status"].as_f64().is_some_and(|s| s < 400.0),
                "Failed resource: {response}"
            );
            let resource = response["url"].as_str().ok_or("Missing resource URL")?;
            assert!(
                resource.starts_with(&url) || resource.starts_with("data:"),
                "External resource: {resource}"
            );
            resources.push(resource);
        }
    }
    let result = json!({
        "status":"passed", "runner":"native Rust",
        "checks":["WebGL + WASM start","relative /repository/ resources","pause","step","reset","algorithm switch","mouse orbit","camera reset","emulated touch orbit","emulated pinch","UI touch capture","viewport rotation","no console errors"],
        "resources":resources, "desktop_boids_fps":{"100_agents":fps100,"200_agents":fps200},
        "browser":"Chromium headless, SwiftShader software WebGL; no physical mobile device"
    });
    let report = serde_json::to_string_pretty(&result)?;
    fs::write(out.join("result.json"), &report)?;
    println!("{report}");
    Ok(())
}
