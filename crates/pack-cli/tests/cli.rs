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
fn search_keyword_mode_prints_source_cards() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(
        root.join("notes/hook.md"),
        "---\ntype: prompt\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.",
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build"])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["search", "훅", "--mode", "keyword"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[keyword]"))
        .stdout(predicate::str::contains("hook#0000"))
        .stdout(predicate::str::contains("클릭을 부르는 훅"));
}

#[test]
fn search_hybrid_requires_real_embed_feature_by_default() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["search", "질문", "--mode", "hybrid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("real-embed"));
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
fn embed_requires_real_embed_feature_by_default() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["embed"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("real-embed"))
        .stderr(predicate::str::contains(
            "cargo build --release --features real-embed",
        ));
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

#[test]
fn serve_once_prints_local_url_and_handles_one_request() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/a.md"), "hello").unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build"])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args([
            "serve",
            "--port",
            "0",
            "--once",
            "--request",
            "GET /api/search?q=hello HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://127.0.0.1:"))
        .stdout(predicate::str::contains("\"note_id\":\"a\""));
}

#[test]
fn open_no_browser_prints_viewer_url() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["open", "--port", "0", "--no-browser", "--print-url"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://127.0.0.1:"));
}

#[test]
fn status_and_list_report_pending_enrichment() {
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
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build", "--no-embed"])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pending_enrichment=1"))
        .stdout(predicate::str::contains("objects.jsonl"));
    assert!(root.join(".pack/objects.jsonl").exists());

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["list", "--pending-enrichment"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pic"))
        .stdout(predicate::str::contains("enrichment=Pending"));
}

#[test]
fn enrich_note_preserves_sidecar_and_makes_caption_searchable() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    let img = dir.path().join("board.png");
    std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["add", img.to_str().unwrap(), "--type", "image"])
        .assert()
        .success();
    std::fs::write(root.join("transcript.txt"), "[00:00] 로컬 지식팩 설명").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args([
            "enrich-note",
            "board",
            "--caption",
            "화이트보드에 온톨로지 그래프가 있다",
            "--tag",
            "ontology",
            "--transcript",
            "transcript.txt",
            "--provider",
            "codex",
            "--model",
            "test-double",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("enrichment 업데이트"));

    let sidecar = std::fs::read_to_string(root.join("notes/board.md")).unwrap();
    assert!(sidecar.contains("캡션을 적어주세요"));
    assert!(sidecar.contains("## AI Caption"));
    assert!(sidecar.contains("화이트보드에 온톨로지 그래프"));
    assert!(sidecar.contains("[00:00] 로컬 지식팩 설명"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["build", "--no-embed"])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["search", "온톨로지"])
        .assert()
        .success()
        .stdout(predicate::str::contains("board#0000"));
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("done_enrichment=1"));
}

#[test]
fn enrich_pending_runs_provider_command_and_rebuilds_search() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    let img = dir.path().join("dashboard.png");
    std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["add", img.to_str().unwrap(), "--type", "image"])
        .assert()
        .success();

    let provider = dir.path().join("provider.py");
    std::fs::write(
        &provider,
        r#"#!/usr/bin/env python3
import json, sys
payload = json.load(sys.stdin)
assert payload["note_id"] == "dashboard"
assert payload["asset_abs_path"].endswith("assets/dashboard.png")
json.dump({
    "caption": "AI worker saw a neon ontology dashboard",
    "tags": ["worker", "ontology"],
    "summary": "Provider command generated this enrichment.",
    "provider": "command-test",
    "model": "fixture"
}, sys.stdout)
"#,
    )
    .unwrap();
    make_executable(&provider);

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args([
            "enrich-pending",
            "--provider-command",
            provider.to_str().unwrap(),
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("processed=1"))
        .stdout(predicate::str::contains("indexed=1"));

    let sidecar = std::fs::read_to_string(root.join("notes/dashboard.md")).unwrap();
    assert!(sidecar.contains("캡션을 적어주세요"));
    assert!(sidecar.contains("AI worker saw a neon ontology dashboard"));
    assert!(root.join(".pack/objects.jsonl").exists());

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["search", "neon"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dashboard#0000"));
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["list", "--pending-enrichment"])
        .assert()
        .success()
        .stdout(predicate::str::contains("객체 없음"));
}

#[test]
fn bundled_fixture_provider_enriches_media() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let provider = repo_root.join("scripts/providers/fixture_media_worker.py");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    let img = dir.path().join("vault.png");
    std::fs::write(&img, [0x89, 0x50, 0x4e, 0x47]).unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["add", img.to_str().unwrap(), "--type", "image"])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args([
            "enrich-pending",
            "--provider-command",
            provider.to_str().unwrap(),
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("processed=1"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["search", "fixture-provider"])
        .assert()
        .success()
        .stdout(predicate::str::contains("vault#0000"));
}

#[test]
fn export_jsonl_markdown_and_mcp_context_include_citations() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    std::fs::write(
        root.join("notes/lecture.md"),
        "---\ntype: lecture\ntitle: 로컬 온톨로지\ntags: [ontology, local]\ncreated: 2026-05-24\nrelated: [\"[[image-board]]\"]\n---\n지식팩을 어디서든 재사용한다.",
    )
    .unwrap();
    std::fs::write(
        root.join("notes/image-board.md"),
        "---\ntype: image\ntitle: 보드 이미지\nasset: assets/board.png\ntags: [visual]\n---\n화이트보드 캡션.",
    )
    .unwrap();
    std::fs::write(root.join("assets/board.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["export", "--format", "jsonl"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""note_id":"lecture""#))
        .stdout(predicate::str::contains(r#""citation""#))
        .stdout(predicate::str::contains(
            r#""asset_path":"assets/board.png""#,
        ));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["export", "--format", "markdown-bundle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# OntoPack Markdown Bundle"))
        .stdout(predicate::str::contains("Citation: `note:lecture`"))
        .stdout(predicate::str::contains("Asset: `assets/board.png`"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["export", "--format", "mcp-context"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""type":"ontopack.mcp_context""#))
        .stdout(predicate::str::contains(r#""context_blocks""#))
        .stdout(predicate::str::contains(r#""citation":"note:image-board""#));
}

#[test]
fn export_can_write_to_output_file() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    let out = dir.path().join("bundle.md");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/a.md"), "---\ntitle: A\n---\nportable body").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args([
            "export",
            "--format",
            "markdown-bundle",
            "--output",
            out.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("export 완료"));

    let body = std::fs::read_to_string(out).unwrap();
    assert!(body.contains("portable body"));
    assert!(body.contains("Citation: `note:a`"));
}

#[test]
fn export_can_copy_referenced_assets_for_portable_bundle() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    let out = dir.path().join("context.jsonl");
    let assets_out = dir.path().join("portable-assets");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();

    std::fs::write(root.join("assets/board.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();
    std::fs::create_dir_all(root.join("assets/.derived/clip")).unwrap();
    std::fs::write(
        root.join("assets/.derived/clip/keyframe-0000.jpg"),
        [0xff, 0xd8, 0xff],
    )
    .unwrap();
    std::fs::write(
        root.join("notes/board.md"),
        "---\ntype: image\ntitle: Board\nasset: assets/board.png\n---\nDerived frame: `assets/.derived/clip/keyframe-0000.jpg`",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args([
            "export",
            "--format",
            "jsonl",
            "--output",
            out.to_str().unwrap(),
            "--copy-assets",
            assets_out.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("assets copied=2"));

    let body = std::fs::read_to_string(out).unwrap();
    assert!(body.contains(r#""asset_path":"assets/board.png""#));
    assert!(assets_out.join("assets/board.png").exists());
    assert!(assets_out
        .join("assets/.derived/clip/keyframe-0000.jpg")
        .exists());
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}
