//! Интеграционные тесты для CLI-сравнивателя.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sample_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("YPbank_formats")
        .join("sample_files")
        .join(name)
}

fn run_compare(args: &[&str]) -> (i32, String, String) {
    let exe = env!("CARGO_BIN_EXE_comparer");
    let output = Command::new(exe)
        .args(args)
        .output()
        .expect("Failed to run comparer binary");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
fn compares_sample_files_across_formats_as_identical() {
    let bin = sample_path("records_example.bin");
    let csv = sample_path("records_example.csv");

    let (code, stdout, stderr) = run_compare(&[
        "--file1",
        bin.to_str().unwrap(),
        "--format1",
        "bin",
        "--file2",
        csv.to_str().unwrap(),
        "--format2",
        "csv",
    ]);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("identical"), "stdout: {stdout}");
}

#[test]
fn reports_mismatch_for_different_files() {
    let csv = sample_path("records_example.csv");
    let tmp_dir = std::env::temp_dir();
    let tmp_file = tmp_dir.join("records_example_short.csv");

    let content = fs::read_to_string(&csv).expect("read sample csv");
    let lines: Vec<&str> = content.lines().collect();
    assert!(
        lines.len() > 1,
        "csv must have header and at least one record"
    );

    let header = lines[0];
    let first = lines[1].to_string();
    let mut parts = first.splitn(8, ',');
    let p0 = parts.next().unwrap_or("");
    let p1 = parts.next().unwrap_or("");
    let p2 = parts.next().unwrap_or("");
    let p3 = parts.next().unwrap_or("");
    let p4 = parts.next().unwrap_or("");
    let p5 = parts.next().unwrap_or("");
    let p6 = parts.next().unwrap_or("");
    let p7 = parts.next().unwrap_or("");

    let new_amount = if p4 == "0" { "1" } else { "0" };
    let mutated_first = format!("{p0},{p1},{p2},{p3},{new_amount},{p5},{p6},{p7}");

    let mut out = String::new();
    out.push_str(header);
    out.push('\n');
    out.push_str(&mutated_first);
    for line in lines.iter().skip(2) {
        out.push('\n');
        out.push_str(line);
    }

    fs::write(&tmp_file, out).expect("write mutated csv");

    let (code, stdout, stderr) = run_compare(&[
        "--file1",
        csv.to_str().unwrap(),
        "--format1",
        "csv",
        "--file2",
        tmp_file.to_str().unwrap(),
        "--format2",
        "csv",
    ]);

    let _ = fs::remove_file(&tmp_file);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("different"), "stdout: {stdout}");
}
