//! The user skill (plans/14-user-skill.md): its frontmatter and links are valid,
//! and its doctor reports a healthy setup and a down herdr. No real herdr: a
//! stub `herdr-mcp` and `claude` on PATH, a release file instead of the GitHub
//! API, and a fake socket that answers `ping`.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn skill() -> String {
    std::fs::read_to_string(root().join("skills/herdr-mcp/SKILL.md")).expect("SKILL.md")
}

#[test]
fn frontmatter_has_name_and_description() {
    let text = skill();
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("---"));
    let front: Vec<&str> = lines.by_ref().take_while(|l| *l != "---").collect();
    assert!(front.contains(&"name: herdr-mcp"), "{front:?}");
    let desc = front
        .iter()
        .find_map(|l| l.strip_prefix("description: "))
        .expect("description");
    assert!(desc.len() > 80, "description names the triggers: {desc}");
    assert!(text.lines().count() <= 80, "the skill stays short");
}

#[test]
fn every_relative_link_resolves() {
    let text = skill();
    let dir = root().join("skills/herdr-mcp");
    let mut checked = 0;
    for part in text.split("](").skip(1) {
        let target = part.split(')').next().expect("link end");
        if target.contains("://") || target.starts_with('#') {
            continue;
        }
        let path = target.split('#').next().unwrap();
        assert!(dir.join(path).exists(), "broken link {target}");
        checked += 1;
    }
    assert!(checked >= 1, "the doctor link is checked");
}

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hm-skill-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tmp dir");
    dir
}

fn stub(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("stub");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// Run the doctor with only the stub dir and the system tools on PATH.
fn doctor(dir: &Path, with_claude: bool, sock: &Path, env: &[(&str, &str)]) -> Output {
    stub(dir, "herdr-mcp", "echo 'herdr-mcp 0.1.0'");
    if with_claude {
        stub(dir, "claude", "echo 'herdr: herdr-mcp - connected'");
    }
    let release = dir.join("latest.json");
    std::fs::write(&release, r#"{"tag_name":"v0.1.0"}"#).expect("release");
    let mut cmd = Command::new("sh");
    cmd.arg(root().join("skills/herdr-mcp/scripts/doctor.sh"))
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", dir.display()))
        .env("HOME", dir)
        .env("HERDR_SOCKET_PATH", sock)
        .env(
            "HERDR_MCP_DOCTOR_RELEASE_URL",
            format!("file://{}", release.display()),
        )
        .env("HERDR_MCP_SECRET_TOKEN", "do-not-print-me");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("run doctor")
}

fn lines(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn doctor_is_healthy_against_a_stubbed_setup() {
    let dir = tmp("ok");
    let sock = dir.join("herdr.sock");
    let listener = UnixListener::bind(&sock).expect("bind");
    let server = std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).expect("read");
            let mut stream = stream;
            stream
                .write_all(b"{\"id\":\"d1\",\"result\":{\"type\":\"pong\",\"protocol\":22}}\n")
                .expect("reply");
        }
    });
    let out = doctor(&dir, true, &sock, &[("HERDR_ENV", "1")]);
    server.join().expect("server");
    let got = lines(&out);
    assert!(out.status.success(), "{got:?}");
    assert!(
        got.iter().all(|l| {
            l.starts_with("ok: ") || l.starts_with("warn: ") || l.starts_with("FAIL: ")
        })
    );
    assert!(!got.iter().any(|l| l.starts_with("FAIL")), "{got:?}");
    for want in [
        "ok: herdr-mcp 0.1.0",
        "ok: version 0.1.0 is the latest",
        "ok: registered in claude",
        "ok: codex not installed, skipped",
        "ok: herdr answers ping",
        "ok: inside a herdr pane",
    ] {
        assert!(got.iter().any(|l| l.starts_with(want)), "{want} in {got:?}");
    }
    assert!(!String::from_utf8_lossy(&out.stdout).contains("do-not-print-me"));
}

#[test]
fn doctor_warns_when_offline_or_outside_a_pane() {
    let dir = tmp("warn");
    let sock = dir.join("herdr.sock");
    let _listener = UnixListener::bind(&sock).expect("bind");
    let out = doctor(
        &dir,
        false,
        &sock,
        &[
            ("HERDR_MCP_DOCTOR_RELEASE_URL", "http://127.0.0.1:1/none"),
            ("HERDR_MCP_READ_ONLY", "1"),
        ],
    );
    let got = lines(&out);
    assert!(
        got.iter().any(|l| l.starts_with("warn: offline, skipped")),
        "{got:?}"
    );
    assert!(
        got.iter()
            .any(|l| l.starts_with("warn: not inside a herdr pane"))
    );
    assert!(
        got.iter()
            .any(|l| l.starts_with("warn: HERDR_MCP_READ_ONLY=1"))
    );
}

#[test]
fn doctor_fails_when_herdr_is_unreachable() {
    let dir = tmp("down");
    let sock = dir.join("missing.sock");
    let out = doctor(&dir, false, &sock, &[]);
    let got = lines(&out);
    assert_eq!(out.status.code(), Some(1), "{got:?}");
    assert!(
        got.iter()
            .any(|l| l.starts_with("FAIL: herdr not reachable at ") && l.contains("missing.sock")),
        "{got:?}"
    );
}

#[test]
fn doctor_fails_when_the_binary_is_missing() {
    let dir = tmp("nobin");
    let sock = dir.join("missing.sock");
    let mut cmd = Command::new("sh");
    let out = cmd
        .arg(root().join("skills/herdr-mcp/scripts/doctor.sh"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &dir)
        .env("HERDR_SOCKET_PATH", &sock)
        .env("HERDR_MCP_DOCTOR_RELEASE_URL", "http://127.0.0.1:1/none")
        .output()
        .expect("run");
    let got = lines(&out);
    assert_eq!(out.status.code(), Some(1), "{got:?}");
    assert!(
        got.iter()
            .any(|l| l.starts_with("FAIL: herdr-mcp not on PATH"))
    );
}
