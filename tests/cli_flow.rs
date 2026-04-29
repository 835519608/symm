use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use serde_json::Value;
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
        .env("SYMM_RM_ACTION", "delete")
        .args(["rm", "demo"])
        .assert()
        .success()
        .stdout(contains("删除成功：demo"));
}

#[test]
fn rm_with_restore_moves_target_back_to_link_path() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target_restore.txt");
    let link = data_root.join("link_restore.txt");
    fs::write(&target, "hello-restore").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "restore-demo")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_RM_ACTION", "restore")
        .args(["rm", "restore-demo"])
        .assert()
        .success()
        .stdout(contains("删除成功并已恢复 target 到 link：restore-demo"));

    assert!(
        !target.exists(),
        "restore 分支应将 target 实体迁移回 link 位置"
    );
    assert_eq!(
        fs::read_to_string(&link).expect("read restored link path entity"),
        "hello-restore"
    );

    let ls_output = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let ls_text = String::from_utf8(ls_output).expect("ls stdout should be valid utf-8 json");
    let ls_json: Value = serde_json::from_str(&ls_text).expect("ls output should be json");
    let items = ls_json.as_array().expect("ls json should be an array");
    assert!(items.is_empty(), "rm 完成后应删除数据库记录，ls 结果应为空");
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
        .success()
        .stdout(contains("正在扫描迁移内容"))
        .stdout(contains("正在快速移动（同盘）"))
        .stdout(contains("正在创建链接"))
        .stdout(contains("正在写入数据库"));

    // 原实体应被移动到 target
    assert_eq!(fs::read_to_string(&target).expect("read moved"), "payload");
    // link 位置应变成软链接（读取内容应等于 target 内容）
    assert_eq!(fs::read_to_string(&link).expect("read via link"), "payload");
}

#[test]
fn add_adopts_existing_link_entity_and_creates_nested_target_parent_dirs() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let link = data_root.join("nested_original.txt");
    let target = data_root
        .join("deep")
        .join("level")
        .join("nested_moved.txt");
    fs::write(&link, "payload").expect("write original");
    assert!(!target.exists());
    assert!(
        !target.parent().expect("target has parent").exists(),
        "nested parent should not pre-exist"
    );

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "adopt-nested")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&target).expect("read moved"), "payload");
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

    let cancel_output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "cancel-add")
        .env("SYMM_ADD_CONFLICT_CHOICE", "cancel")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run cancel add");
    assert!(!cancel_output.status.success());
    let cancel_err =
        String::from_utf8(cancel_output.stderr).expect("cancel stderr should be valid utf-8 json");
    let cancel_json: Value = serde_json::from_str(&cancel_err).expect("stderr should be json");
    assert_eq!(cancel_json["code"], "invalid_argument");

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

    let ls_output = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let ls_text = String::from_utf8(ls_output).expect("ls stdout should be valid utf-8 json");
    let ls_json: Value = serde_json::from_str(&ls_text).expect("ls output should be json");
    let items = ls_json.as_array().expect("ls json should be an array");
    assert_eq!(items.len(), 1, "same link should upsert instead of insert");
    assert_eq!(items[0]["name"], "v2");

    // Windows 下 JSON 中 target_path 可能是规范化后的 \\?\ 前缀绝对路径，避免直接比字符串。
    assert_eq!(
        fs::read_to_string(&link).expect("read updated link target"),
        "b"
    );
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
        .stdout(contains("\"name\": \"second\""));

    // 避免 Windows 8.3 短路径与 \\?\ 规范路径导致的字符串不一致。
    assert_eq!(
        fs::read_to_string(&link).expect("read managed link"),
        "same"
    );
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

#[test]
fn add_when_link_is_locked_and_user_cancels_fails_before_mutation() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("locked_target.txt");
    let link = data_root.join("locked_link.txt");
    fs::write(&link, "payload").expect("write link entity");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "locked")
        .env("SYMM_ADD_LOCK_CHOICE", "cancel")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run locked add");

    assert!(!output.status.success());
    let err = String::from_utf8(output.stderr).expect("stderr should be utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("已取消解除占用")
    );
    assert_eq!(fs::read_to_string(&link).expect("read link"), "payload");
    assert!(!target.exists(), "target should remain absent after cancel");
}

#[test]
fn add_when_link_is_locked_and_unlock_succeeds_continues_normally() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("unlock_target.txt");
    let link = data_root.join("unlock_link.txt");
    fs::write(&link, "payload").expect("write link entity");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "unlock")
        .env("SYMM_ADD_LOCK_CHOICE", "unlock")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("正在检查 link 占用"))
        .stdout(contains("检测到占用进程"))
        .stdout(contains("正在结束全部占用进程"))
        .stdout(contains("正在重新确认占用状态"))
        .stdout(contains("正在扫描迁移内容"));

    assert_eq!(fs::read_to_string(&target).expect("read target"), "payload");
    assert_eq!(fs::read_to_string(&link).expect("read link"), "payload");
}

#[test]
fn add_when_link_is_locked_and_unlock_still_leaves_locks_fails() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("still_locked_target.txt");
    let link = data_root.join("still_locked_link.txt");
    fs::write(&link, "payload").expect("write link entity");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "still-locked")
        .env("SYMM_ADD_LOCK_CHOICE", "unlock")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .env("SYMM_TEST_LOCK_CLEAR_ON_KILL", "false")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run locked add");

    assert!(!output.status.success());
    let err = String::from_utf8(output.stderr).expect("stderr should be utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "io_error");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("仍被占用")
    );
    assert_eq!(fs::read_to_string(&link).expect("read link"), "payload");
    assert!(
        !target.exists(),
        "target should remain absent after failed unlock"
    );
}

#[test]
fn add_with_invalid_conflict_choice_env_fails_fast() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_invalid_conflict.txt");
    let link = data_root.join("link_invalid_conflict.txt");
    fs::write(&target, "t").expect("write target");
    fs::write(&link, "l").expect("write link");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "invalid-choice")
        .env("SYMM_ADD_CONFLICT_CHOICE", "bad_value")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run add");
    assert!(!output.status.success());

    let err = String::from_utf8(output.stderr).expect("stderr utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("SYMM_ADD_CONFLICT_CHOICE")
    );
}

#[test]
fn add_with_invalid_lock_choice_env_fails_fast() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_invalid_lock.txt");
    let link = data_root.join("link_invalid_lock.txt");
    fs::write(&link, "payload").expect("write link");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "invalid-lock")
        .env("SYMM_ADD_LOCK_CHOICE", "bad_value")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run add");
    assert!(!output.status.success());

    let err = String::from_utf8(output.stderr).expect("stderr utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("SYMM_ADD_LOCK_CHOICE")
    );
}

#[test]
fn add_name_conflict_rolls_back_created_link() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target1 = data_root.join("target_name_conflict_1.txt");
    let link1 = data_root.join("link_name_conflict_1.txt");
    let target2 = data_root.join("target_name_conflict_2.txt");
    let link2 = data_root.join("link_name_conflict_2.txt");
    fs::write(&target1, "a").expect("write target1");
    fs::write(&target2, "b").expect("write target2");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "dup-name")
        .args(["add", &link1.to_string_lossy(), &target1.to_string_lossy()])
        .assert()
        .success();

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "dup-name")
        .args(["add", &link2.to_string_lossy(), &target2.to_string_lossy()])
        .output()
        .expect("run conflicting add");
    assert!(!output.status.success());
    assert!(
        !link2.exists(),
        "db 冲突后应清理刚创建的 link，避免文件系统与数据库不一致"
    );
}

#[test]
fn ls_status_filters_broken_and_missing() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target_ok = data_root.join("target_ok.txt");
    let target_broken = data_root.join("target_broken.txt");
    let link_ok = data_root.join("link_ok.txt");
    let link_broken = data_root.join("link_broken.txt");
    let link_missing = data_root.join("link_missing.txt");
    fs::write(&target_ok, "ok").expect("write target ok");
    fs::write(&target_broken, "broken").expect("write target broken");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "ok-item")
        .args([
            "add",
            &link_ok.to_string_lossy(),
            &target_ok.to_string_lossy(),
        ])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "broken-item")
        .args([
            "add",
            &link_broken.to_string_lossy(),
            &target_broken.to_string_lossy(),
        ])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_ADD_NAME", "missing-item")
        .args([
            "add",
            &link_missing.to_string_lossy(),
            &target_ok.to_string_lossy(),
        ])
        .assert()
        .success();

    fs::remove_file(&target_broken).expect("remove broken target");
    fs::remove_file(&link_missing).expect("remove missing link");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--status", "broken"])
        .assert()
        .success()
        .stdout(contains("broken-item"))
        .stdout(predicates::str::contains("ok-item").not())
        .stdout(predicates::str::contains("missing-item").not());

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--status", "missing"])
        .assert()
        .success()
        .stdout(contains("missing-item"))
        .stdout(predicates::str::contains("ok-item").not())
        .stdout(predicates::str::contains("broken-item").not());
}
