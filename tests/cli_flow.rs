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
        .stdout(contains("创建成功：demo"));

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
        .stdout(contains("名称: demo"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "demo"])
        .assert()
        .success()
        .stdout(contains("删除成功：demo"));
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
        .stdout(contains("\"name\":\"demo2\""))
        .stdout(contains("\"status\":\"ok\""));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "demo2", "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\": \"demo2\""))
        .stdout(contains("\"status\": \"ok\""));
}

#[test]
fn add_adopts_existing_link_entity_when_target_missing() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("moved.txt");
    let link = data_root.join("original.txt");

    // link 先存在实体，target 不存在
    fs::write(&link, "payload").expect("write original");
    assert!(!target.exists());
    assert!(link.exists());

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args([
            "add",
            "adopt",
            &target.to_string_lossy(),
            &link.to_string_lossy(),
        ])
        .assert()
        .success();

    // 原实体应被移动到 target
    assert_eq!(fs::read_to_string(&target).expect("read moved"), "payload");
    // link 位置应变成软链接（读取内容应等于 target 内容）
    assert_eq!(fs::read_to_string(&link).expect("read via link"), "payload");
}
