use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;

fn cmd() -> Command {
    Command::cargo_bin("symm").expect("binary exists")
}

#[test]
fn add_then_ls_then_show_then_rm() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target.txt");
    let link = data_root.join("link.txt");
    fs::write(&target, "hello").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args([
            "add",
            "demo",
            &target.to_string_lossy(),
            &link.to_string_lossy(),
        ])
        .assert()
        .success()
        .stdout(contains("created: demo"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("demo"))
        .stdout(contains("ok"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "demo"])
        .assert()
        .success()
        .stdout(contains("name: demo"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "demo"])
        .assert()
        .success()
        .stdout(contains("removed: demo"));
}

#[test]
fn ls_json_and_show_json_work() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target2.txt");
    let link = data_root.join("link2.txt");
    fs::write(&target, "hello").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args([
            "add",
            "demo2",
            &target.to_string_lossy(),
            &link.to_string_lossy(),
        ])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\": \"demo2\""))
        .stdout(contains("\"status\": \"ok\""));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "demo2", "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\": \"demo2\""))
        .stdout(contains("\"status\": \"ok\""));
}
