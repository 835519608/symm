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
        .env("SYMM_ADD_NAME", "demo")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("name: demo"));

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
        .env("SYMM_ADD_NAME", "demo2")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
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
        .env("SYMM_ADD_NAME", "adopt")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    // 原实体应被移动到 target
    assert_eq!(fs::read_to_string(&target).expect("read moved"), "payload");
    // link 位置应变成软链接（读取内容应等于 target 内容）
    assert_eq!(fs::read_to_string(&link).expect("read via link"), "payload");
}

#[test]
fn add_when_target_and_link_both_exist_keep_link() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_keep_link.txt");
    let link = data_root.join("link_keep_link.txt");
    fs::write(&target, "from-target").expect("write target");
    fs::write(&link, "from-link").expect("write link entity");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "keep-link")
        .env("SYMM_ADD_CONFLICT_CHOICE", "link")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&target).expect("read kept link payload"),
        "from-link"
    );
    assert_eq!(
        fs::read_to_string(&link).expect("read through new symlink"),
        "from-link"
    );
}

#[test]
fn add_when_target_and_link_both_exist_keep_target() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_keep_target.txt");
    let link = data_root.join("link_keep_target.txt");
    fs::write(&target, "stay-target").expect("write target");
    fs::write(&link, "drop-link-entity").expect("write link entity");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "keep-target")
        .env("SYMM_ADD_CONFLICT_CHOICE", "target")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&target).expect("read kept target payload"),
        "stay-target"
    );
    assert_eq!(
        fs::read_to_string(&link).expect("read through recreated symlink"),
        "stay-target"
    );
}

#[test]
fn add_when_target_and_link_both_exist_cancel() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_cancel.txt");
    let link = data_root.join("link_cancel.txt");
    fs::write(&target, "keep-target").expect("write target");
    fs::write(&link, "keep-link").expect("write link entity");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "cancel-add")
        .env("SYMM_ADD_CONFLICT_CHOICE", "cancel")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .failure()
        .stderr(contains("\"code\":\"invalid_argument\""));

    assert_eq!(
        fs::read_to_string(&target).expect("read target"),
        "keep-target"
    );
    assert_eq!(fs::read_to_string(&link).expect("read link"), "keep-link");
}

#[test]
fn add_same_link_updates_record_instead_of_inserting_new_one() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target_a = data_root.join("target_a.txt");
    let target_b = data_root.join("target_b.txt");
    let link = data_root.join("same_link.txt");
    fs::write(&target_a, "a").expect("write target a");
    fs::write(&target_b, "b").expect("write target b");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "v1")
        .args(["add", &link.to_string_lossy(), &target_a.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "v2")
        .env("SYMM_ADD_SYMLINK_CONFLICT_CHOICE", "retarget")
        .args(["add", &link.to_string_lossy(), &target_b.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\":\"v2\""))
        .stdout(contains(target_b.to_string_lossy().as_ref()));
}

#[test]
fn add_existing_symlink_pointing_to_same_target_is_managed_without_conflict_prompt() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("same_target.txt");
    let link = data_root.join("same_target_link.txt");
    fs::write(&target, "same").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "first")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    // link 已经是指向 target 的软链接，再次 add 应直接纳管/更新，不应进入冲突交互
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "second")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", &link.to_string_lossy(), "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\": \"second\""))
        .stdout(contains(target.to_string_lossy().as_ref()));
}

#[test]
fn add_existing_symlink_pointing_elsewhere_can_retarget() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target_a = data_root.join("retarget_a.txt");
    let target_b = data_root.join("retarget_b.txt");
    let link = data_root.join("retarget_link.txt");
    fs::write(&target_a, "a").expect("write target a");
    fs::write(&target_b, "b").expect("write target b");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "retarget")
        .args(["add", &link.to_string_lossy(), &target_a.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "retarget")
        .env("SYMM_ADD_SYMLINK_CONFLICT_CHOICE", "retarget")
        .args(["add", &link.to_string_lossy(), &target_b.to_string_lossy()])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&link).expect("read retargeted link"),
        "b"
    );
}
