//! Интеграционные тесты для CLI-конвертера.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_converter")
}

fn unique_path(name: &str) -> std::path::PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("ypbank_cli_{name}_{pid}_{ts}"))
}

fn sample_txt() -> String {
    [
        "TX_ID: 1",
        "TX_TYPE: DEPOSIT",
        "FROM_USER_ID: 0",
        "TO_USER_ID: 42",
        "AMOUNT: 100",
        "TIMESTAMP: 1700000000000",
        "STATUS: SUCCESS",
        "DESCRIPTION: \"hello\"",
        "",
    ]
    .join("\n")
}

fn expected_csv() -> String {
    [
        "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION",
        "1,DEPOSIT,0,42,100,1700000000000,SUCCESS,\"hello\"",
        "",
    ]
    .join("\n")
}

#[test]
fn converts_txt_to_csv_with_files() {
    let input_path = unique_path("input.txt");
    let output_path = unique_path("output.csv");
    fs::write(&input_path, sample_txt()).unwrap();

    let status = Command::new(bin_path())
        .args([
            "--in",
            "txt",
            "--out",
            "csv",
            "-i",
            input_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success(), "process exited with {status}");

    let out = fs::read_to_string(&output_path).unwrap();
    assert_eq!(out, expected_csv());
}

#[test]
fn converts_txt_to_csv_via_stdio() {
    let mut child = Command::new(bin_path())
        .args(["--in", "txt", "--out", "csv"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(sample_txt().as_bytes()).unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "process exited with {}",
        output.status
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected_csv());
}

#[test]
fn unknown_argument_fails() {
    let output = Command::new(bin_path()).arg("--nope").output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown argument"));
}
