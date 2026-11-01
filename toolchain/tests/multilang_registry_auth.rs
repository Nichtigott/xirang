#![cfg(feature = "cli")]
//! `xirang registry fetch --auth-env <variable>`: a private mirror needs a credential, and a
//! credential must not travel through argv where a process listing would show it. The token is
//! read from the named environment variable and written into curl's config file on stdin.
//! `xirang registry fetch --auth-env <变量>`：私有镜像需要凭据，而凭据不能经过 argv（进程列表会
//! 显示它）。令牌从点名的环境变量读取，写进 stdin 上的 curl 配置文件。

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-registry-auth-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn invoke_variable(directory: &Path, arguments: &[&str], variable: Option<&str>) -> Output {
    let mut command = Command::new(binary());
    command.args(arguments).current_dir(directory);
    if let Some(variable) = variable {
        command.envs(std::env::vars().filter(|(name, _)| name == variable));
    }
    command.output().unwrap()
}

fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn sha(path: &Path) -> String {
    let output = Command::new("sha256sum").arg(path).output().unwrap();
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

/// Serve `directory` on a loopback port, answering only requests that carry `Bearer <token>`.
fn serve(directory: &Path, token: &str) -> (Child, u16) {
    let mut child = Command::new("python3")
        .args([
            "-c",
            "import http.server, os, socketserver\n\
             expected = \"Bearer \" + os.environ[\"EXPECTED_TOKEN\"]\n\
             class Handler(http.server.SimpleHTTPRequestHandler):\n\
             \x20   def log_message(self, *args): pass\n\
             \x20   def do_GET(self):\n\
             \x20       if self.headers.get(\"Authorization\") != expected:\n\
             \x20           self.send_error(401, \"unauthorized\")\n\
             \x20           return\n\
             \x20       super().do_GET()\n\
             httpd = socketserver.TCPServer((\"127.0.0.1\", 0), Handler)\n\
             print(httpd.server_address[1], flush=True)\n\
             httpd.serve_forever()\n",
        ])
        .env("EXPECTED_TOKEN", token)
        .current_dir(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut line = String::new();
    std::io::BufReader::new(stdout)
        .read_line(&mut line)
        .unwrap();
    (child, line.trim().parse().unwrap())
}

/// A one-package mirror and an index naming its manifest.
fn mirror(root: &Path) -> PathBuf {
    let serve = root.join("serve");
    let version = serve.join("greeter").join("0.1.0");
    std::fs::create_dir_all(&version).unwrap();
    std::fs::write(
        version.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"greeter\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    let file = "greeter/0.1.0/xirang.toml";
    let index = format!(
        "{{\"protocol\":1,\"packages\":[{{\"name\":\"greeter\",\"version\":\"0.1.0\",\"files\":[{{\"path\":\"{file}\",\"sha256\":\"{}\"}}]}}]}}",
        sha(&serve.join(file))
    );
    std::fs::write(serve.join("index.json"), index).unwrap();
    serve
}

fn fetch(directory: &Path, port: u16, digest: &str, into: &Path, variable: Option<&str>) -> Output {
    let mut arguments = vec![
        "registry".to_owned(),
        "fetch".to_owned(),
        "--index".to_owned(),
        format!("http://127.0.0.1:{port}/index.json"),
        "--sha256".to_owned(),
        digest.to_owned(),
        "--into".to_owned(),
        into.display().to_string(),
    ];
    if let Some(variable) = variable {
        arguments.push("--auth-env".to_owned());
        arguments.push(variable.to_owned());
    }
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    invoke_variable(directory, &borrowed, variable)
}

#[test]
fn a_token_from_the_environment_is_sent_through_stdin_and_never_echoed() {
    if !available("python3") || !available("curl") {
        println!("skipping: python3 or curl is not available");
        return;
    }
    let token = "a\"b\\c";
    let root = scratch("ok");
    let tree = mirror(&root);
    let digest = sha(&tree.join("index.json"));
    let (mut server, port) = serve(&tree, token);
    // The variable is set for the child only; the test process does not export it.
    let into = root.join("mirror");
    let fetched = Command::new(binary())
        .args([
            "registry",
            "fetch",
            "--index",
            &format!("http://127.0.0.1:{port}/index.json"),
            "--sha256",
            &digest,
            "--into",
            into.to_str().unwrap(),
            "--auth-env",
            "XIRANG_TEST_TOKEN",
        ])
        .current_dir(&root)
        .env("XIRANG_TEST_TOKEN", token)
        .output()
        .unwrap();
    assert!(fetched.status.success(), "{}", both(&fetched));
    assert!(into.join("greeter/0.1.0/xirang.toml").is_file());
    assert!(!both(&fetched).contains(token));
    server.kill().unwrap();
    let _ = server.wait();
}

#[test]
fn a_wrong_token_is_refused_and_the_token_is_not_echoed() {
    if !available("python3") || !available("curl") {
        println!("skipping: python3 or curl is not available");
        return;
    }
    let token = "right-token";
    let root = scratch("wrong");
    let tree = mirror(&root);
    let digest = sha(&tree.join("index.json"));
    let (mut server, port) = serve(&tree, token);
    let into = root.join("mirror");
    let mut command = Command::new(binary());
    command
        .args([
            "registry",
            "fetch",
            "--index",
            &format!("http://127.0.0.1:{port}/index.json"),
            "--sha256",
            &digest,
            "--into",
            into.to_str().unwrap(),
            "--auth-env",
            "XIRANG_TEST_TOKEN",
        ])
        .current_dir(&root)
        .env("XIRANG_TEST_TOKEN", "wrong-token");
    let refused = command.output().unwrap();
    assert!(!refused.status.success());
    assert!(both(&refused).contains("401"), "{}", both(&refused));
    assert!(!both(&refused).contains("wrong-token"));
    assert!(!into.exists());
    // Without the flag the same mirror refuses too.
    let anonymous = fetch(&root, port, &digest, &into, None);
    assert!(!anonymous.status.success());
    assert!(both(&anonymous).contains("401"), "{}", both(&anonymous));
    server.kill().unwrap();
    let _ = server.wait();
}

#[test]
fn a_missing_or_corrupt_token_is_refused_before_any_request() {
    if !available("curl") {
        println!("skipping: curl is not available");
        return;
    }
    let root = scratch("missing");
    let into = root.join("mirror");
    let digest = "0".repeat(64);
    let missing = invoke_variable(
        &root,
        &[
            "registry",
            "fetch",
            "--index",
            "http://127.0.0.1:1/index.json",
            "--sha256",
            &digest,
            "--into",
            into.to_str().unwrap(),
            "--auth-env",
            "XIRANG_VARIABLE_THAT_IS_NOT_SET",
        ],
        None,
    );
    assert!(!missing.status.success());
    assert!(
        both(&missing).contains("XIRANG_VARIABLE_THAT_IS_NOT_SET"),
        "{}",
        both(&missing)
    );
    let mut command = Command::new(binary());
    command
        .args([
            "registry",
            "fetch",
            "--index",
            "http://127.0.0.1:1/index.json",
            "--sha256",
            &digest,
            "--into",
            into.to_str().unwrap(),
            "--auth-env",
            "XIRANG_TEST_TOKEN",
        ])
        .current_dir(&root)
        .env("XIRANG_TEST_TOKEN", "line\nbreak");
    let corrupt = command.output().unwrap();
    assert!(!corrupt.status.success());
    assert!(
        both(&corrupt).contains("control character"),
        "{}",
        both(&corrupt)
    );
    assert!(!into.exists());
}
