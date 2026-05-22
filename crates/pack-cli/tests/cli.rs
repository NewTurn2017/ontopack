use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn init_creates_pack() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("초기화"));
    assert!(root.join("pack.toml").exists());
    assert!(root.join("notes").is_dir());
}

#[test]
fn init_escapes_pack_name_for_toml() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("bad\"pack");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build"])
        .assert()
        .success();
}

#[test]
fn add_markdown_creates_note() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    let src = dir.path().join("hello.md");
    std::fs::write(&src, "---\ntype: prompt\ntitle: 헬로\n---\n본문").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["add", src.to_str().unwrap()])
        .assert()
        .success();

    assert!(root.join("notes/hello.md").exists());
}

#[test]
fn add_refuses_to_overwrite_existing_note() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/hello.md"), "기존 본문").unwrap();
    let src = dir.path().join("hello.md");
    std::fs::write(&src, "새 본문").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["add", src.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("이미 존재"));
}

#[test]
fn add_binary_creates_asset_and_sidecar() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    let img = dir.path().join("pic.png");
    std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["add", img.to_str().unwrap(), "--type", "image"])
        .assert()
        .success();

    assert!(root.join("assets/pic.png").exists());
    let sidecar = std::fs::read_to_string(root.join("notes/pic.md")).unwrap();
    assert!(sidecar.contains("type: image"));
    assert!(sidecar.contains("asset: assets/pic.png"));
}

#[test]
fn add_binary_escapes_sidecar_yaml() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    let img = dir.path().join("a: b.png");
    std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["add", img.to_str().unwrap(), "--type", "image:still"])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build"])
        .assert()
        .success();
}

#[test]
fn end_to_end_build_and_search() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    std::fs::write(
        root.join("notes/whale.md"),
        "---\ntype: prompt\ntitle: 고래\ntags: [sea]\n---\n바다 고래 이야기",
    )
    .unwrap();
    std::fs::write(
        root.join("notes/car.md"),
        "---\ntype: prompt\ntitle: 자동차\n---\n도로 자동차 이야기",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["search", "고래"])
        .assert()
        .success()
        .stdout(predicate::str::contains("고래"))
        .stdout(predicate::str::contains("자동차").not());
}

#[test]
fn process_imports_inbox_files() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("_inbox/memo.md"), "메모").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["process"])
        .assert()
        .success()
        .stdout(predicate::str::contains("처리 완료"));

    assert!(root.join("notes/memo.md").exists());
    assert!(!root.join("_inbox/memo.md").exists());
}

#[test]
fn build_incremental_reports_skips_on_second_run() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/a.md"), "본문").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build", "--incremental"])
        .assert()
        .success()
        .stdout(predicate::str::contains("indexed=1"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build", "--incremental"])
        .assert()
        .success()
        .stdout(predicate::str::contains("skipped=1"));
}

#[test]
fn build_no_embed_keeps_keyword_only_build_offline() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/a.md"), "본문").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build", "--no-embed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("인덱스 빌드 완료"));
}

#[test]
fn end_to_end_process_build_and_search() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(
        root.join("_inbox/hook.md"),
        "---\ntype: prompt\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["process"])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build", "--incremental"])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["search", "훅"])
        .assert()
        .success()
        .stdout(predicate::str::contains("썸네일 훅"));
}
