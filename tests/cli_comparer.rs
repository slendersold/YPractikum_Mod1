//! Интеграционные тесты для CLI-сравнивателя.

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ypbank_{name}_{ts}.tmp"));
    fs::write(&path, contents).expect("write temp file");
    path
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
    let csv = write_temp_file(
        "a",
        "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
1,DEPOSIT,0,1,100,123,SUCCESS,\"Test\"",
    );
    let txt = write_temp_file(
        "b",
        "TX_ID: 1\n\
TX_TYPE: DEPOSIT\n\
FROM_USER_ID: 0\n\
TO_USER_ID: 1\n\
AMOUNT: 100\n\
TIMESTAMP: 123\n\
STATUS: SUCCESS\n\
DESCRIPTION: \"Test\"\n",
    );

    let (code, stdout, stderr) = run_compare(&[
        "--file1",
        csv.to_str().unwrap(),
        "--format1",
        "csv",
        "--file2",
        txt.to_str().unwrap(),
        "--format2",
        "txt",
    ]);

    let _ = fs::remove_file(&csv);
    let _ = fs::remove_file(&txt);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("identical"), "stdout: {stdout}");
}

#[test]
fn reports_mismatch_for_different_files() {
    let csv = write_temp_file(
        "c",
        "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
1,DEPOSIT,0,1,100,123,SUCCESS,\"Test\"",
    );
    let csv_changed = write_temp_file(
        "d",
        "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
1,DEPOSIT,0,1,101,123,SUCCESS,\"Test\"",
    );

    let (code, stdout, stderr) = run_compare(&[
        "--file1",
        csv.to_str().unwrap(),
        "--format1",
        "csv",
        "--file2",
        csv_changed.to_str().unwrap(),
        "--format2",
        "csv",
    ]);

    let _ = fs::remove_file(&csv);
    let _ = fs::remove_file(&csv_changed);

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("different"), "stdout: {stdout}");
}
