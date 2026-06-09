use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn cmd() -> Command {
    let mut command = Command::cargo_bin("symm-cli").expect("binary exists");
    command.env("SYMM_TEST_SKIP_PRIVILEGED_LOCK", "1");
    command
}

#[cfg(windows)]
fn create_junction(target: &Path, link: &Path) {
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .expect("run mklink");
    assert!(status.success(), "mklink /J should succeed");
}

fn create_file_symlink(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).expect("symlink");

    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link).expect("symlink");
}

fn assert_same_missing_target_path(actual: &Path, expected: &Path) {
    if actual == expected {
        return;
    }
    assert_eq!(actual.file_name(), expected.file_name());
    assert_eq!(
        dunce::canonicalize(actual.parent().expect("actual parent")).expect("actual parent"),
        dunce::canonicalize(expected.parent().expect("expected parent")).expect("expected parent")
    );
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
        .env("SYMM_LINK_OP_NAME", "demo")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("名称：demo"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("demo"))
        .stdout(contains("正常"));

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
        .stdout(contains("已删除链接关系：demo"));
}

#[test]
fn cli_parse_errors_are_rendered_as_json_but_help_stays_text() {
    let output = cmd()
        .args(["ls", "--status", "nope"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let text = String::from_utf8(output).expect("stderr utf8");
    let json: Value = serde_json::from_str(&text).expect("parse error should be json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message")
            .contains("状态无效")
    );

    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("软链接管理命令行工具"));
}

#[test]
fn ls_invalid_limits_are_rejected_as_json_parse_errors() {
    for raw in ["0", "-1", "abc", "4294967296"] {
        let output = cmd()
            .args(["ls", "--limit", raw])
            .assert()
            .failure()
            .get_output()
            .stderr
            .clone();
        let text = String::from_utf8(output).expect("stderr utf8");
        let json: Value = serde_json::from_str(&text).expect("parse error should be json");
        assert_eq!(json["code"], "invalid_argument");
        assert!(
            json["message"]
                .as_str()
                .expect("message")
                .contains("limit 无效"),
            "unexpected message for {raw}: {json:?}"
        );
    }
}

#[test]
fn add_normalizes_lexically_equivalent_link_paths() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target.txt");
    fs::write(&target, "hello").expect("write target");

    cmd()
        .current_dir(&data_root)
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "first")
        .args(["add", "link.txt", "target.txt"])
        .assert()
        .success();

    cmd()
        .current_dir(&data_root)
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "second")
        .args(["add", "./link.txt", "target.txt"])
        .assert()
        .success();

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("json stdout");
    let json: Value = serde_json::from_str(&text).expect("ls json");
    let items = json.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "second");
}

#[test]
fn rm_by_list_index_after_delete_middle_row() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    for (name, n) in [("a", 1), ("b", 2), ("c", 3)] {
        let target = data_root.join(format!("target_idx_{n}.txt"));
        let link = data_root.join(format!("link_idx_{n}.txt"));
        fs::write(&target, "x").expect("write target");
        cmd()
            .env("SYMM_HOME", &symm_home)
            .env("SYMM_LINK_OP_NAME", name)
            .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
            .assert()
            .success();
    }

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "2"])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "2"])
        .assert()
        .success()
        .stdout(contains("名称: c"));
}

#[test]
fn rm_multiple_list_indices_deletes_requested_rows() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    for name in ["a", "b", "c"] {
        let target = data_root.join(format!("target_{name}.txt"));
        let link = data_root.join(format!("link_{name}.txt"));
        fs::write(&target, "x").expect("write target");
        cmd()
            .env("SYMM_HOME", &symm_home)
            .env("SYMM_LINK_OP_NAME", name)
            .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
            .assert()
            .success();
    }

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "1", "3"])
        .assert()
        .success()
        .stdout(contains("共 2 条"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\":\"b\""))
        .stdout(predicates::str::contains("\"name\":\"a\"").not())
        .stdout(predicates::str::contains("\"name\":\"c\"").not());
}

#[test]
fn rm_multiple_selectors_deletes_all() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target1 = data_root.join("target_rm_multi_1.txt");
    let link1 = data_root.join("link_rm_multi_1.txt");
    let target2 = data_root.join("target_rm_multi_2.txt");
    let link2 = data_root.join("link_rm_multi_2.txt");
    fs::write(&target1, "a").expect("write target1");
    fs::write(&target2, "b").expect("write target2");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "rm-a")
        .args(["add", &link1.to_string_lossy(), &target1.to_string_lossy()])
        .assert()
        .success();
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "rm-b")
        .args(["add", &link2.to_string_lossy(), &target2.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "rm-a", "rm-b"])
        .assert()
        .success()
        .stdout(contains("共 2 条"))
        .stdout(contains("rm-a"))
        .stdout(contains("rm-b"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .stdout(contains("[]"));
}

#[cfg(unix)]
#[test]
fn rm_multiple_partial_failure_returns_failure() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    let protected_dir = data_root.join("protected");
    let normal_dir = data_root.join("normal");
    fs::create_dir_all(&protected_dir).expect("create protected root");
    fs::create_dir_all(&normal_dir).expect("create normal root");

    let protected_target = data_root.join("protected_target.txt");
    let protected_link = protected_dir.join("protected_link.txt");
    let normal_target = data_root.join("normal_target.txt");
    let normal_link = normal_dir.join("normal_link.txt");
    fs::write(&protected_target, "protected").expect("write protected target");
    fs::write(&normal_target, "normal").expect("write normal target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "rm-protected")
        .args([
            "add",
            &protected_link.to_string_lossy(),
            &protected_target.to_string_lossy(),
        ])
        .assert()
        .success();
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "rm-normal")
        .args([
            "add",
            &normal_link.to_string_lossy(),
            &normal_target.to_string_lossy(),
        ])
        .assert()
        .success();

    let original_mode = fs::metadata(&protected_dir)
        .expect("protected metadata")
        .permissions()
        .mode();
    fs::set_permissions(&protected_dir, fs::Permissions::from_mode(0o555))
        .expect("make protected root readonly");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "rm-protected", "rm-normal"])
        .assert()
        .failure()
        .stdout(contains("已删除链接关系：rm-normal"))
        .stdout(contains("失败：rm-protected"))
        .stderr(contains("\"code\": \"batch_failure\""));

    fs::set_permissions(&protected_dir, fs::Permissions::from_mode(original_mode))
        .expect("restore protected root permissions");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\":\"rm-protected\""))
        .stdout(predicates::str::contains("\"name\":\"rm-normal\"").not());
}

#[test]
fn rm_missing_selector_fails_before_prompting_for_mode() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .write_stdin("")
        .args(["rm", "missing"])
        .assert()
        .failure()
        .stderr(contains("\"code\": \"not_found\""));
}

#[cfg(windows)]
#[test]
fn add_existing_junction_records_junction_kind() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target_junction");
    let link = data_root.join("link_junction");
    fs::create_dir(&target).expect("create target dir");
    create_junction(&target, &link);

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "junction-demo")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("json stdout");
    let json: Value = serde_json::from_str(&text).expect("ls json");
    let items = json.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "junction-demo");
    assert_eq!(items[0]["link_kind"], "junction");
    assert_eq!(items[0]["status"], "ok");
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
        .env("SYMM_LINK_OP_NAME", "restore-demo")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["restore", "restore-demo"])
        .assert()
        .success()
        .stdout(contains("已恢复实体位置：restore-demo"));

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
        .env("SYMM_LINK_OP_NAME", "demo2")
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
fn ls_json_limit_and_offset_stream_requested_page() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    for name in ["first", "second", "third"] {
        let target = data_root.join(format!("target_{name}.txt"));
        let link = data_root.join(format!("link_{name}.txt"));
        fs::write(&target, "x").expect("write target");
        cmd()
            .env("SYMM_HOME", &symm_home)
            .env("SYMM_LINK_OP_NAME", name)
            .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
            .assert()
            .success();
    }

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json", "--limit", "1", "--offset", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("json stdout");
    let json: Value = serde_json::from_str(&text).expect("ls json");
    let items = json.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["index"], 2);
    assert_eq!(items[0]["name"], "second");
}

#[test]
fn ls_table_defaults_to_first_page_and_keeps_global_indices() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    for n in 1..=101 {
        let name = format!("item-{n:03}");
        let target = data_root.join(format!("target_{n:03}.txt"));
        let link = data_root.join(format!("link_{n:03}.txt"));
        fs::write(&target, "x").expect("write target");
        cmd()
            .env("SYMM_HOME", &symm_home)
            .env("SYMM_LINK_OP_NAME", &name)
            .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
            .assert()
            .success();
    }

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("item-100"))
        .stdout(predicates::str::contains("item-101").not())
        .stdout(contains("下一页：symm-cli ls --limit 100 --offset 100"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--limit", "1", "--offset", "100"])
        .assert()
        .success()
        .stdout(contains("101"))
        .stdout(contains("item-101"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "101"])
        .assert()
        .success()
        .stdout(contains("名称: item-101"));
}

#[test]
fn adopt_moves_existing_link_entity_when_target_missing() {
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
        .env("SYMM_LINK_OP_NAME", "adopt")
        .args(["adopt", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("正在扫描："))
        .stdout(contains("正在同盘移动："))
        .stdout(contains("正在创建软链："))
        .stdout(contains("正在保存记录："));

    // 原实体应被移动到 target
    assert_eq!(fs::read_to_string(&target).expect("read moved"), "payload");
    // link 位置应变成软链接（读取内容应等于 target 内容）
    assert_eq!(fs::read_to_string(&link).expect("read via link"), "payload");
}

#[test]
fn adopt_moves_existing_link_entity_and_creates_nested_target_parent_dirs() {
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
        .env("SYMM_LINK_OP_NAME", "adopt-nested")
        .args(["adopt", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&target).expect("read moved"), "payload");
    assert_eq!(fs::read_to_string(&link).expect("read via link"), "payload");
}

#[test]
fn add_when_target_and_link_both_exist_fails_without_mutation() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_keep_link.txt");
    let link = data_root.join("link_keep_link.txt");
    fs::write(&target, "from-target").expect("write target");
    fs::write(&link, "from-link").expect("write link entity");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "keep-link")
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
            .contains("adopt")
    );

    assert_eq!(
        fs::read_to_string(&target).expect("read target payload"),
        "from-target"
    );
    assert_eq!(
        fs::read_to_string(&link).expect("read link payload"),
        "from-link"
    );
}

#[test]
fn adopt_when_target_exists_fails_without_mutation() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_keep_target.txt");
    let link = data_root.join("link_keep_target.txt");
    fs::write(&target, "stay-target").expect("write target");
    fs::write(&link, "drop-link-entity").expect("write link entity");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "keep-target")
        .args(["adopt", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run adopt");
    assert!(!output.status.success());
    let err = String::from_utf8(output.stderr).expect("stderr utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("target 路径已存在")
    );

    assert_eq!(
        fs::read_to_string(&target).expect("read kept target payload"),
        "stay-target"
    );
    assert_eq!(
        fs::read_to_string(&link).expect("read unchanged link entity"),
        "drop-link-entity"
    );
}

#[test]
fn point_when_link_is_entity_fails_without_mutation() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_cancel.txt");
    let link = data_root.join("link_cancel.txt");
    fs::write(&target, "keep-target").expect("write target");
    fs::write(&link, "keep-link").expect("write link entity");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "cancel-add")
        .args(["point", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run point");
    assert!(!output.status.success());
    let err = String::from_utf8(output.stderr).expect("stderr should be valid utf-8 json");
    let json: Value = serde_json::from_str(&err).expect("stderr should be json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("不是链接")
    );

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
        .env("SYMM_LINK_OP_NAME", "v1")
        .args(["add", &link.to_string_lossy(), &target_a.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "v2")
        .args([
            "point",
            &link.to_string_lossy(),
            &target_b.to_string_lossy(),
        ])
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
        .env("SYMM_LINK_OP_NAME", "first")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    // link 已经是指向 target 的软链接，再次 add 应直接纳管/更新，不应进入冲突交互
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "second")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "second", "--json"])
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
fn point_existing_symlink_pointing_elsewhere_can_repoint() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target_a = data_root.join("point_a.txt");
    let target_b = data_root.join("point_b.txt");
    let link = data_root.join("point_link.txt");
    fs::write(&target_a, "a").expect("write target a");
    fs::write(&target_b, "b").expect("write target b");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "point-demo")
        .args(["add", &link.to_string_lossy(), &target_a.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "point-demo")
        .args([
            "point",
            &link.to_string_lossy(),
            &target_b.to_string_lossy(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(&link).expect("read repointed link"), "b");
}

#[test]
fn adopt_when_link_is_locked_and_user_cancels_fails_before_mutation() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("locked_target.txt");
    let link = data_root.join("locked_link.txt");
    fs::write(&link, "payload").expect("write link entity");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "locked")
        .env("SYMM_LINK_OP_LOCK_CHOICE", "cancel")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args(["adopt", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run locked adopt");

    assert!(!output.status.success());
    let err = String::from_utf8(output.stderr).expect("stderr should be utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("链接位置仍被占用，已取消")
    );
    assert_eq!(fs::read_to_string(&link).expect("read link"), "payload");
    assert!(!target.exists(), "target should remain absent after cancel");
}

#[test]
fn adopt_when_link_is_locked_and_unlock_succeeds_continues_normally() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("unlock_target.txt");
    let link = data_root.join("unlock_link.txt");
    fs::write(&link, "payload").expect("write link entity");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "unlock")
        .env("SYMM_LINK_OP_LOCK_CHOICE", "unlock")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args(["adopt", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("正在检查链接是否被占用"))
        .stdout(contains("检测到占用"))
        .stdout(contains("正在结束占用进程"))
        .stdout(contains("等待程序释放文件"))
        .stdout(contains("正在扫描："));

    assert_eq!(fs::read_to_string(&target).expect("read target"), "payload");
    assert_eq!(fs::read_to_string(&link).expect("read link"), "payload");
}

#[test]
fn adopt_when_link_is_locked_and_unlock_still_leaves_locks_fails() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("still_locked_target.txt");
    let link = data_root.join("still_locked_link.txt");
    fs::write(&link, "payload").expect("write link entity");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "still-locked")
        .env("SYMM_LINK_OP_LOCK_CHOICE", "unlock")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .env("SYMM_TEST_LOCK_CLEAR_ON_KILL", "false")
        .args(["adopt", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run locked adopt");

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
fn point_existing_broken_symlink_can_repoint() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let old_target = data_root.join("old_broken_target.txt");
    let new_target = data_root.join("new_point_target.txt");
    let link = data_root.join("broken_point_link.txt");
    fs::write(&old_target, "old").expect("write old target");
    fs::write(&new_target, "new").expect("write new target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "broken-point")
        .args([
            "add",
            &link.to_string_lossy(),
            &old_target.to_string_lossy(),
        ])
        .assert()
        .success();
    fs::remove_file(&old_target).expect("break existing symlink");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "broken-point")
        .args([
            "point",
            &link.to_string_lossy(),
            &new_target.to_string_lossy(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&link).expect("read repointed link"),
        "new"
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
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "broken-point");

    let actual_target = items[0]["target_path"]
        .as_str()
        .expect("target_path should be a string");
    assert_eq!(
        dunce::canonicalize(Path::new(actual_target)).expect("canonicalize stored target"),
        dunce::canonicalize(&new_target).expect("canonicalize new target")
    );
}

#[test]
fn point_broken_symlink_still_checks_link_lock_before_mutation() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let old_target = data_root.join("locked_broken_old.txt");
    let new_target = data_root.join("locked_broken_new.txt");
    let link = data_root.join("locked_broken_link.txt");
    fs::write(&old_target, "old").expect("write old target");
    fs::write(&new_target, "new").expect("write new target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "locked-broken")
        .args([
            "add",
            &link.to_string_lossy(),
            &old_target.to_string_lossy(),
        ])
        .assert()
        .success();
    let before_ls = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let before_text = String::from_utf8(before_ls).expect("before json stdout");
    let before_json: Value = serde_json::from_str(&before_text).expect("before json");
    let expected_target_path = before_json[0]["target_path"]
        .as_str()
        .expect("before target_path")
        .to_string();
    fs::remove_file(&old_target).expect("break existing symlink");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "locked-broken")
        .env("SYMM_LINK_OP_LOCK_CHOICE", "cancel")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args([
            "point",
            &link.to_string_lossy(),
            &new_target.to_string_lossy(),
        ])
        .output()
        .expect("run locked broken point");

    assert!(!output.status.success());
    let err = String::from_utf8(output.stderr).expect("stderr should be utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "invalid_argument");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("链接位置仍被占用，已取消")
    );
    assert_same_missing_target_path(
        &fs::read_link(&link).expect("read broken link target"),
        &old_target,
    );
    let after_ls = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let after_text = String::from_utf8(after_ls).expect("after json stdout");
    let after_json: Value = serde_json::from_str(&after_text).expect("after json");
    assert_eq!(after_json[0]["target_path"], expected_target_path);
    assert_ne!(
        after_json[0]["target_path"].as_str().expect("after target"),
        new_target.to_string_lossy()
    );
}

#[test]
fn add_existing_symlink_pointing_elsewhere_fails_with_point_hint() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target_a = data_root.join("target_existing_link_a.txt");
    let target_b = data_root.join("target_existing_link_b.txt");
    let link = data_root.join("link_existing_link.txt");
    fs::write(&target_a, "a").expect("write target a");
    fs::write(&target_b, "b").expect("write target b");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "point-hint")
        .args(["add", &link.to_string_lossy(), &target_a.to_string_lossy()])
        .assert()
        .success();

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "invalid-choice")
        .args(["add", &link.to_string_lossy(), &target_b.to_string_lossy()])
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
            .contains("point")
    );
    assert_eq!(fs::read_to_string(&link).expect("read original link"), "a");
}

#[test]
fn add_missing_target_preempts_invalid_lock_choice_env() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_invalid_lock.txt");
    let link = data_root.join("link_invalid_lock.txt");
    fs::write(&link, "payload").expect("write link");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "invalid-lock")
        .env("SYMM_LINK_OP_LOCK_CHOICE", "bad_value")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .output()
        .expect("run add");
    assert!(!output.status.success());

    let err = String::from_utf8(output.stderr).expect("stderr utf8");
    let json: Value = serde_json::from_str(&err).expect("stderr json");
    assert_eq!(json["code"], "target_not_found");
    assert!(
        json["message"]
            .as_str()
            .expect("message string")
            .contains("target_invalid_lock")
    );
}

#[test]
fn link_op_lock_choice_rejects_undocumented_unlock_all_alias() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_unlock_all.txt");
    let link = data_root.join("link_unlock_all.txt");
    fs::write(&link, "payload").expect("write link entity");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "unlock-all")
        .env("SYMM_LINK_OP_LOCK_CHOICE", "unlock_all")
        .env("SYMM_TEST_LOCK_PATHS", link.to_string_lossy().to_string())
        .args(["adopt", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .failure()
        .stderr(contains("SYMM_LINK_OP_LOCK_CHOICE 无效"))
        .stderr(contains("unlock_all"));

    assert_eq!(fs::read_to_string(&link).expect("read link"), "payload");
    assert!(!target.exists(), "target should remain absent");
}

#[test]
fn add_pure_digit_name_is_stored_with_link_prefix() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target_digit_prefix.txt");
    let link = data_root.join("link_digit_prefix.txt");
    fs::write(&target, "x").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "42")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success()
        .stdout(contains("名称「42」已改为「link-42」"))
        .stdout(contains("名称：link-42"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "link-42"])
        .assert()
        .success()
        .stdout(contains("名称: link-42"));
}

#[test]
fn add_name_conflict_fails_before_creating_link() {
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
        .env("SYMM_LINK_OP_NAME", "dup-name")
        .args(["add", &link1.to_string_lossy(), &target1.to_string_lossy()])
        .assert()
        .success();

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "dup-name")
        .args(["add", &link2.to_string_lossy(), &target2.to_string_lossy()])
        .output()
        .expect("run conflicting add");
    assert!(!output.status.success());
    assert!(
        fs::symlink_metadata(&link2).is_err(),
        "name 冲突应在创建 link 前失败"
    );
}

#[test]
fn adopt_name_conflict_fails_before_moving_entity() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target1 = data_root.join("target_adopt_conflict_1.txt");
    let link1 = data_root.join("link_adopt_conflict_1.txt");
    fs::write(&target1, "managed").expect("write target1");
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "dup-adopt")
        .args(["add", &link1.to_string_lossy(), &target1.to_string_lossy()])
        .assert()
        .success();

    let link2 = data_root.join("link_adopt_conflict_2.txt");
    let target2 = data_root.join("target_adopt_conflict_2.txt");
    fs::write(&link2, "keep-link-entity").expect("write link entity");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "dup-adopt")
        .args([
            "adopt",
            &link2.to_string_lossy(),
            &target2.to_string_lossy(),
        ])
        .assert()
        .failure()
        .stderr(contains("名称冲突"));

    assert_eq!(
        fs::read_to_string(&link2).expect("link entity should stay"),
        "keep-link-entity"
    );
    assert!(
        fs::symlink_metadata(&target2).is_err(),
        "adopt target should not be created on name conflict"
    );
}

#[test]
fn point_name_conflict_fails_before_replacing_link() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target1 = data_root.join("target_point_conflict_1.txt");
    let link1 = data_root.join("link_point_conflict_1.txt");
    fs::write(&target1, "managed").expect("write target1");
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "dup-point")
        .args(["add", &link1.to_string_lossy(), &target1.to_string_lossy()])
        .assert()
        .success();

    let target2 = data_root.join("target_point_conflict_2.txt");
    let target3 = data_root.join("target_point_conflict_3.txt");
    let link2 = data_root.join("link_point_conflict_2.txt");
    fs::write(&target2, "old").expect("write target2");
    fs::write(&target3, "new").expect("write target3");
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "point-owner")
        .args(["add", &link2.to_string_lossy(), &target2.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "dup-point")
        .args([
            "point",
            &link2.to_string_lossy(),
            &target3.to_string_lossy(),
        ])
        .assert()
        .failure()
        .stderr(contains("名称冲突"));

    assert_eq!(
        fs::read_to_string(&link2).expect("old link should still point at target2"),
        "old"
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
    let link_missing_2 = data_root.join("link_missing_2.txt");
    fs::write(&target_ok, "ok").expect("write target ok");
    fs::write(&target_broken, "broken").expect("write target broken");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "ok-item")
        .args([
            "add",
            &link_ok.to_string_lossy(),
            &target_ok.to_string_lossy(),
        ])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "broken-item")
        .args([
            "add",
            &link_broken.to_string_lossy(),
            &target_broken.to_string_lossy(),
        ])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "missing-item")
        .args([
            "add",
            &link_missing.to_string_lossy(),
            &target_ok.to_string_lossy(),
        ])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "missing-item-2")
        .args([
            "add",
            &link_missing_2.to_string_lossy(),
            &target_ok.to_string_lossy(),
        ])
        .assert()
        .success();

    fs::remove_file(&target_broken).expect("remove broken target");
    fs::remove_file(&link_missing).expect("remove missing link");
    fs::remove_file(&link_missing_2).expect("remove second missing link");

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

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .args([
            "ls", "--json", "--status", "missing", "--limit", "1", "--offset", "1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).expect("json stdout");
    let json: Value = serde_json::from_str(&text).expect("ls status json");
    let items = json.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "missing-item-2");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--status", "missing", "--limit", "1"])
        .assert()
        .success()
        .stdout(contains("missing-item"))
        .stdout(predicates::str::contains("missing-item-2").not())
        .stdout(contains(
            "下一页：symm-cli ls --limit 1 --offset 1 --status missing",
        ));
}

#[cfg(unix)]
#[test]
fn ls_status_unknown_is_not_reported_as_missing() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    let protected_dir = data_root.join("protected");
    fs::create_dir_all(&protected_dir).expect("create protected dir");
    let target = data_root.join("target_unknown.txt");
    let link = protected_dir.join("link_unknown.txt");
    fs::write(&target, "payload").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "unknown-item")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    let original_mode = fs::metadata(&protected_dir)
        .expect("protected metadata")
        .permissions()
        .mode();
    fs::set_permissions(&protected_dir, fs::Permissions::from_mode(0o000))
        .expect("remove protected dir permissions");

    let output = cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json", "--status", "unknown"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    fs::set_permissions(&protected_dir, fs::Permissions::from_mode(original_mode))
        .expect("restore protected dir permissions");

    let text = String::from_utf8(output).expect("json stdout");
    let json: Value = serde_json::from_str(&text).expect("ls unknown json");
    let items = json.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "unknown-item");
    assert_eq!(items[0]["status"], "unknown");
    assert!(items[0]["status_error"].as_str().is_some());
}

#[cfg(unix)]
#[test]
fn rm_and_restore_fail_on_unknown_status_before_mutation() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    let protected_dir = data_root.join("protected-strict");
    fs::create_dir_all(&protected_dir).expect("create protected dir");
    let target = data_root.join("target_unknown_strict.txt");
    let link = protected_dir.join("link_unknown_strict.txt");
    fs::write(&target, "payload").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "unknown-strict")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    let original_mode = fs::metadata(&protected_dir)
        .expect("protected metadata")
        .permissions()
        .mode();
    fs::set_permissions(&protected_dir, fs::Permissions::from_mode(0o000))
        .expect("remove protected dir permissions");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "unknown-strict"])
        .assert()
        .failure()
        .stderr(contains("\"code\": \"io_error\""));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["restore", "unknown-strict"])
        .assert()
        .failure()
        .stderr(contains("\"code\": \"io_error\""));

    fs::set_permissions(&protected_dir, fs::Permissions::from_mode(original_mode))
        .expect("restore protected dir permissions");

    assert!(
        fs::symlink_metadata(&link).is_ok(),
        "strict probe failure should not remove link"
    );
    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("unknown-strict"));
}

#[test]
fn add_without_positional_args_uses_env_paths() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target_env.txt");
    let link = data_root.join("link_env.txt");
    fs::write(&target, "hello").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_LINK", &link)
        .env("SYMM_LINK_OP_TARGET", &target)
        .env("SYMM_LINK_OP_NAME", "env-add")
        .args(["add"])
        .assert()
        .success()
        .stdout(contains("env-add"));
}

#[test]
fn restore_on_stale_fails_and_keeps_stale_record_while_restoring_others() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target_ok = data_root.join("target_ok.txt");
    let link_ok = data_root.join("link_ok.txt");
    fs::write(&target_ok, "ok").expect("write target");
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "ok-item")
        .args([
            "add",
            &link_ok.to_string_lossy(),
            &target_ok.to_string_lossy(),
        ])
        .assert()
        .success();

    let target_stale = data_root.join("target_stale_batch.txt");
    let link_stale = data_root.join("link_stale_batch");
    fs::write(&target_stale, "stale").expect("write target");
    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "stale-item")
        .args([
            "add",
            &link_stale.to_string_lossy(),
            &target_stale.to_string_lossy(),
        ])
        .assert()
        .success();
    fs::remove_file(&link_stale).expect("remove symlink");
    fs::create_dir_all(&link_stale).expect("replace with dir");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["restore", "stale-item", "ok-item"])
        .assert()
        .failure()
        .stdout(contains("已恢复实体位置：ok-item"))
        .stdout(contains("失败：stale-item"))
        .stderr(contains("\"code\": \"batch_failure\""));

    assert!(link_stale.exists());
    assert!(link_ok.exists());
    assert_eq!(
        fs::read_to_string(&link_ok).expect("read restored link"),
        "ok"
    );

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("stale-item"))
        .stdout(predicates::str::contains("ok-item").not());
}

#[test]
fn restore_on_broken_fails_and_keeps_record() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_broken_restore.txt");
    let link = data_root.join("link_broken_restore.txt");
    fs::write(&target, "payload").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "broken-restore")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();
    fs::remove_file(&target).expect("remove target to break symlink");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["restore", "broken-restore"])
        .assert()
        .failure()
        .stderr(contains("\"code\": \"invalid_argument\""))
        .stderr(contains("target 不存在"));

    assert!(
        fs::symlink_metadata(&link).is_ok(),
        "broken link should remain when restore fails before mutation"
    );
    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("broken-restore"));
}

#[cfg(unix)]
#[test]
fn restore_failure_after_unlink_keeps_record_and_reports_retry_boundary() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    let target_dir = data_root.join("target-dir");
    let link_dir = data_root.join("link-dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    fs::create_dir_all(&link_dir).expect("create link dir");

    let target = target_dir.join("target_restore_retry.txt");
    let link = link_dir.join("link_restore_retry.txt");
    fs::write(&target, "payload").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "retry-restore")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    let original_mode = fs::metadata(&target_dir)
        .expect("target dir metadata")
        .permissions()
        .mode();
    fs::set_permissions(&target_dir, fs::Permissions::from_mode(0o555))
        .expect("make target dir readonly");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["restore", "retry-restore"])
        .assert()
        .failure()
        .stderr(contains("\"code\": \"filesystem_applied_but_record_kept\""))
        .stderr(contains("link 已移除"))
        .stderr(contains("可修复原因后重试 restore"));

    fs::set_permissions(&target_dir, fs::Permissions::from_mode(original_mode))
        .expect("restore target dir permissions");

    assert!(
        fs::symlink_metadata(&link).is_err(),
        "link should stay removed after post-unlink restore failure"
    );
    assert!(target.exists(), "target should remain for retry");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("retry-restore"))
        .stdout(contains("链接没了"));

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["restore", "retry-restore"])
        .assert()
        .success()
        .stdout(contains("已恢复实体位置：retry-restore"));

    assert_eq!(
        fs::read_to_string(&link).expect("read restored link path entity"),
        "payload"
    );
    assert!(!target.exists(), "target should be moved back to link path");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls", "--json"])
        .assert()
        .success()
        .stdout(contains("\"name\":\"retry-restore\"").not());
}

#[test]
fn rm_broken_entry_deletes_link_and_db_record() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_broken_rm.txt");
    let link = data_root.join("link_broken_rm.txt");
    fs::write(&target, "payload").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "broken-rm")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();
    fs::remove_file(&target).expect("remove target to break symlink");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "broken-rm"])
        .assert()
        .success()
        .stdout(contains("已删除链接关系：broken-rm"));

    assert!(
        fs::symlink_metadata(&link).is_err(),
        "broken link should be removed by rm"
    );
    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("broken-rm").not());
}

#[test]
fn rm_missing_entry_deletes_db_record_only() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_missing_rm.txt");
    let link = data_root.join("link_missing_rm.txt");
    fs::write(&target, "payload").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "missing-rm")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();
    fs::remove_file(&link).expect("remove managed link");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "missing-rm"])
        .assert()
        .success()
        .stdout(contains("已删除链接关系：missing-rm"));

    assert!(
        fs::symlink_metadata(&link).is_err(),
        "missing link path should stay missing"
    );
    assert_eq!(
        fs::read_to_string(&target).expect("target should stay"),
        "payload"
    );
    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("missing-rm").not());
}

#[test]
fn rm_stale_entry_deletes_db_and_keeps_non_symlink_path() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_stale_rm.txt");
    let link = data_root.join("link_stale_rm");
    fs::write(&target, "x").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "stale-rm")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    fs::remove_file(&link).expect("remove symlink");
    fs::create_dir_all(&link).expect("replace with dir");
    fs::write(link.join("child.txt"), "keep").expect("write child");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "stale-rm"])
        .assert()
        .success()
        .stdout(contains("已删除"))
        .stdout(contains("只删记录"));

    assert!(link.join("child.txt").exists());

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("stale-rm").not());
}

#[test]
fn rm_drift_entry_deletes_db_and_keeps_current_link() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_drift_rm.txt");
    let other = data_root.join("other_drift_rm.txt");
    let link = data_root.join("link_drift_rm.txt");
    fs::write(&target, "record-target").expect("write target");
    fs::write(&other, "current-target").expect("write other");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "drift-rm")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    fs::remove_file(&link).expect("remove managed link");
    create_file_symlink(&other, &link);

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["rm", "drift-rm"])
        .assert()
        .success()
        .stdout(contains("已删除"))
        .stdout(contains("只删记录"));

    assert_eq!(fs::read_link(&link).expect("read drift link"), other);
    assert_eq!(
        fs::read_to_string(&link).expect("read through drift link"),
        "current-target"
    );

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("drift-rm").not());
}

#[test]
fn restore_on_drift_fails_and_keeps_record_and_current_link() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_drift_restore.txt");
    let other = data_root.join("other_drift_restore.txt");
    let link = data_root.join("link_drift_restore.txt");
    fs::write(&target, "record-target").expect("write target");
    fs::write(&other, "current-target").expect("write other");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "drift-restore")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    fs::remove_file(&link).expect("remove managed link");
    create_file_symlink(&other, &link);

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["restore", "drift-restore"])
        .assert()
        .failure()
        .stderr(contains("\"code\": \"invalid_argument\""))
        .stderr(contains("指向记录以外的位置"));

    assert_eq!(fs::read_link(&link).expect("read drift link"), other);
    assert_eq!(
        fs::read_to_string(&link).expect("read through drift link"),
        "current-target"
    );
    assert_eq!(
        fs::read_to_string(&target).expect("record target remains"),
        "record-target"
    );

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("drift-restore"))
        .stdout(contains("指向不对"));
}

#[test]
fn ls_shows_stale_status_when_link_no_longer_symlink() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");

    let target = data_root.join("target_stale.txt");
    let link = data_root.join("link_stale.txt");
    fs::write(&target, "x").expect("write target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "stale-item")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    fs::remove_file(&link).expect("remove symlink");
    fs::write(&link, "no longer symlink").expect("replace with file");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(contains("stale-item"))
        .stdout(contains("链接类型不符"))
        .stdout(predicates::str::contains("正常").not());
}

#[test]
fn show_by_list_index() {
    let temp = tempdir().expect("temp dir");
    let symm_home = temp.path().join("symm_home");
    let data_root = temp.path().join("data");
    fs::create_dir_all(&data_root).expect("create data root");
    let target = data_root.join("target_id.txt");
    let link = data_root.join("link_id.txt");
    fs::write(&target, "x").expect("target");

    cmd()
        .env("SYMM_HOME", &symm_home)
        .env("SYMM_LINK_OP_NAME", "id-demo")
        .args(["add", &link.to_string_lossy(), &target.to_string_lossy()])
        .assert()
        .success();

    cmd()
        .env("SYMM_HOME", &symm_home)
        .args(["show", "1", "--json"])
        .assert()
        .success()
        .stdout(contains("\"index\": 1"))
        .stdout(contains("\"name\": \"id-demo\""));
}
