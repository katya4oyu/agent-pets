use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--version") | Some("cli-info") => {
            println!("{}", navi_lib::cli_info());
        }
        Some("hook") => {
            if let Some(source) = std::env::args().nth(2) {
                run_hook_cli(&source);
            }
        }
        _ => {}
    }
}

fn inject_terminal_env(raw: &str) -> Option<String> {
    let mut payload: serde_json::Value = serde_json::from_str(raw).ok()?;
    let obj = payload.as_object_mut()?;

    if let Ok(v) = std::env::var("TERM_PROGRAM") {
        obj.entry("terminal_program")
            .or_insert_with(|| serde_json::Value::String(v));
    }

    let session_id = std::env::var("TERM_SESSION_ID")
        .or_else(|_| std::env::var("ITERM_SESSION_ID"))
        .or_else(|_| std::env::var("WEZTERM_PANE"))
        .or_else(|_| std::env::var("KITTY_WINDOW_ID"))
        .ok();
    if let Some(v) = session_id {
        obj.entry("terminal_session_id")
            .or_insert_with(|| serde_json::Value::String(v));
    }

    serde_json::to_string(&payload).ok()
}

fn run_hook_cli(source: &str) {
    if !navi_lib::is_valid_hook_source(source) {
        return;
    }
    let Some(port) = navi_lib::read_navi_port() else {
        return;
    };

    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() {
        return;
    }

    let body = inject_terminal_env(&raw).unwrap_or(raw);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(150)) else {
        return;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_millis(150)));
    let request = format!(
        "POST /events/{source} HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );
    let _ = stream.write_all(request.as_bytes());
}
