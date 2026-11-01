#![cfg(feature = "cli")]
//! `xirang registry fetch`: a mirror is a tree of `xirang.toml` directories, so a fetcher has to
//! prove what it wrote is the tree the caller pinned. The index names every file's sha256, the
//! caller pins the index's own sha256, and nothing lands until every byte matches.
//! `xirang registry fetch`：镜像是 `xirang.toml` 目录树，因此取回器必须证明写下的树就是调用方
//! 钉住的那棵树。索引给每个文件的 sha256，调用方钉住索引自己的 sha256；全部字节吻合之前不落盘。

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-registry-fetch-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
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

/// Serve `directory` on a loopback port; the returned child owns the server process.
fn serve(directory: &Path) -> (Child, u16) {
    let mut child = Command::new("python3")
        .args([
            "-c",
            "import http.server, socketserver\n\
             class Handler(http.server.SimpleHTTPRequestHandler):\n\
             \x20   def log_message(self, *args): pass\n\
             httpd = socketserver.TCPServer((\"127.0.0.1\", 0), Handler)\n\
             print(httpd.server_address[1], flush=True)\n\
             httpd.serve_forever()\n",
        ])
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
    let port = line.trim().parse().unwrap();
    (child, port)
}

/// A mirror holding one package with one version, and an index naming both files.
fn mirror(root: &Path) -> PathBuf {
    let serve = root.join("serve");
    let version = serve.join("greeter").join("0.1.0");
    std::fs::create_dir_all(version.join("src")).unwrap();
    std::fs::write(
        version.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"greeter\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        version.join("src/greeter.cpp"),
        "#include <cstdint>\nstd::int64_t value() { return 1; }\n",
    )
    .unwrap();
    let files = ["greeter/0.1.0/xirang.toml", "greeter/0.1.0/src/greeter.cpp"];
    let entries: Vec<String> = files
        .iter()
        .map(|file| {
            format!(
                "{{\"path\":\"{file}\",\"sha256\":\"{}\"}}",
                sha(&serve.join(file))
            )
        })
        .collect();
    let index = format!(
        "{{\"protocol\":1,\"packages\":[{{\"name\":\"greeter\",\"version\":\"0.1.0\",\"files\":[{}]}}]}}",
        entries.join(",")
    );
    std::fs::write(serve.join("index.json"), &index).unwrap();
    serve
}

fn fetch(directory: &Path, port: u16, digest: &str, into: &Path) -> Output {
    invoke(
        directory,
        &[
            "registry",
            "fetch",
            "--index",
            &format!("http://127.0.0.1:{port}/index.json"),
            "--sha256",
            digest,
            "--into",
            into.to_str().unwrap(),
        ],
    )
}

#[test]
fn a_pinned_index_is_fetched_and_the_mirror_checks_out() {
    if !available("python3") || !available("curl") {
        println!("skipping: python3 or curl is not available");
        return;
    }
    let root = scratch("ok");
    let tree = mirror(&root);
    let digest = sha(&tree.join("index.json"));
    let (mut server, port) = serve(&tree);
    let into = root.join("mirror");
    let fetched = fetch(&root, port, &digest, &into);
    assert!(fetched.status.success(), "{}", both(&fetched));
    assert!(into.join("greeter/0.1.0/xirang.toml").is_file());
    assert!(into.join("greeter/0.1.0/src/greeter.cpp").is_file());
    let checked = invoke(&root, &["registry", "check", into.to_str().unwrap()]);
    assert!(checked.status.success(), "{}", both(&checked));
    // A mirror is written once.
    let again = fetch(&root, port, &digest, &into);
    assert!(!again.status.success());
    assert!(both(&again).contains("already exists"), "{}", both(&again));
    server.kill().unwrap();
    let _ = server.wait();
}

#[test]
fn an_index_that_does_not_match_its_pinned_digest_is_refused() {
    if !available("python3") || !available("curl") {
        println!("skipping: python3 or curl is not available");
        return;
    }
    let root = scratch("digest");
    let tree = mirror(&root);
    let (mut server, port) = serve(&tree);
    let into = root.join("mirror");
    let refused = fetch(&root, port, &"0".repeat(64), &into);
    assert!(!refused.status.success());
    assert!(
        both(&refused).contains("not the pinned"),
        "{}",
        both(&refused)
    );
    assert!(!into.exists());
    server.kill().unwrap();
    let _ = server.wait();
}

#[test]
fn a_file_that_does_not_match_the_index_is_refused_and_staging_is_removed() {
    if !available("python3") || !available("curl") {
        println!("skipping: python3 or curl is not available");
        return;
    }
    let root = scratch("tamper");
    let tree = mirror(&root);
    let digest = sha(&tree.join("index.json"));
    // Change the bytes after the index was written: the file's sha256 no longer matches.
    std::fs::write(
        tree.join("greeter/0.1.0/src/greeter.cpp"),
        "#include <cstdint>\nstd::int64_t value() { return 2; }\n",
    )
    .unwrap();
    let (mut server, port) = serve(&tree);
    let into = root.join("mirror");
    let refused = fetch(&root, port, &digest, &into);
    assert!(!refused.status.success());
    assert!(
        both(&refused).contains("not the index's"),
        "{}",
        both(&refused)
    );
    assert!(!into.exists());
    let leftovers: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().into_owned();
            name.starts_with(".mirror").then_some(name)
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "staging was left behind: {leftovers:?}"
    );
    server.kill().unwrap();
    let _ = server.wait();
}

#[test]
fn plain_http_is_refused_except_from_loopback_and_unknown_schemes_are_refused() {
    if !available("curl") {
        println!("skipping: curl is not available");
        return;
    }
    let root = scratch("scheme");
    let into = root.join("mirror");
    let digest = "0".repeat(64);
    let remote = invoke(
        &root,
        &[
            "registry",
            "fetch",
            "--index",
            "http://example.com/index.json",
            "--sha256",
            &digest,
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(!remote.status.success());
    assert!(both(&remote).contains("loopback"), "{}", both(&remote));
    let unknown = invoke(
        &root,
        &[
            "registry",
            "fetch",
            "--index",
            "ftp://example.com/index.json",
            "--sha256",
            &digest,
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(!unknown.status.success());
    assert!(both(&unknown).contains("https"), "{}", both(&unknown));
    assert!(!into.exists());
}
