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

#[cfg(not(feature = "real-embed"))]
#[test]
fn serve_semantic_requires_real_embed_build() {
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
        .args([
            "serve",
            "--port",
            "0",
            "--once",
            "--semantic",
            "--request",
            "GET /api/capabilities HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("real-embed feature"));
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
fn duplicates_reports_matching_note_bodies() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/a.md"), "---\ntitle: A\n---\n중복 본문").unwrap();
    std::fs::write(root.join("notes/b.md"), "---\ntitle: B\n---\n중복   본문").unwrap();
    std::fs::write(root.join("notes/c.md"), "다른 본문").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["duplicates"])
        .assert()
        .success()
        .stdout(predicate::str::contains("중복 후보: groups=1"))
        .stdout(predicate::str::contains("- a [note]"))
        .stdout(predicate::str::contains("- b [note]"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["duplicates", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""note_id": "a""#))
        .stdout(predicate::str::contains(r#""note_id": "b""#));
}

#[test]
fn orphans_reports_unlinked_notes() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/a.md"), "A [[b]]").unwrap();
    std::fs::write(root.join("notes/b.md"), "B").unwrap();
    std::fs::write(root.join("notes/c.md"), "---\ntitle: C\n---\n외톨이").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["orphans"])
        .assert()
        .success()
        .stdout(predicate::str::contains("외톨이 노트: count=1"))
        .stdout(predicate::str::contains("- c [note]"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["orphans", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""note_id": "c""#));
}

#[test]
fn gaps_reports_missing_wikilink_targets() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(root.join("notes/a.md"), "A [[b]] [[missing]]").unwrap();
    std::fs::write(root.join("notes/b.md"), "B [[missing]] [[other]]").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["gaps"])
        .assert()
        .success()
        .stdout(predicate::str::contains("깨진 링크: count=3"))
        .stdout(predicate::str::contains("a -> missing"))
        .stdout(predicate::str::contains("b -> other"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["gaps", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""missing_target": "missing""#));
}

#[test]
fn topics_reports_tag_topic_map() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(
        root.join("notes/a.md"),
        "---\ntags: [ontology, lecture]\n---\nA",
    )
    .unwrap();
    std::fs::write(
        root.join("notes/b.md"),
        "---\ntags: [ontology, agent]\n---\nB",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["topics", "--min-count", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("토픽맵: topics=1 edges=0"))
        .stdout(predicate::str::contains("topic ontology count=2"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["topics", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""topic": "ontology""#))
        .stdout(predicate::str::contains(r#""source": "agent""#));
}

#[test]
fn recommend_reports_unlinked_notes_with_shared_tags() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("p");
    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", root.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(
        root.join("notes/a.md"),
        "---\ntitle: A\ntags: [ontology, lecture]\nrelated: [b]\n---\nA",
    )
    .unwrap();
    std::fs::write(
        root.join("notes/b.md"),
        "---\ntitle: B\ntags: [ontology, lecture]\n---\nB",
    )
    .unwrap();
    std::fs::write(
        root.join("notes/c.md"),
        "---\ntitle: C\ntags: [ontology, lecture, agent]\n---\nC",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["recommend", "a", "-k", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("관련 노트 추천: count=1"))
        .stdout(predicate::str::contains("a -> c score=2"))
        .stdout(predicate::str::contains("a -> b").not());

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&root)
        .args(["recommend", "a", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""candidate_id": "c""#));
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

#[test]
fn import_jsonl_roundtrips_exported_context_and_assets() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let context = dir.path().join("context.jsonl");
    let assets = dir.path().join("bundle-assets");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("assets/board.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();
    std::fs::write(
        source.join("notes/board.md"),
        "---\ntype: image\ntitle: Board\ntags: [visual]\nasset: assets/board.png\n---\n복원 가능한 온톨로지 보드.",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args([
            "export",
            "--format",
            "jsonl",
            "--output",
            context.to_str().unwrap(),
            "--copy-assets",
            assets.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args([
            "import",
            context.to_str().unwrap(),
            "--format",
            "jsonl",
            "--asset-root",
            assets.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("import 완료: notes=1 assets=1"));

    assert!(restored.join("notes/board.md").exists());
    assert!(restored.join("assets/board.png").exists());
    let restored_note = std::fs::read_to_string(restored.join("notes/board.md")).unwrap();
    assert!(restored_note.starts_with("---\n"));
    assert!(restored_note.contains("\ntype: image\n"));
    assert!(restored_note.contains("title: Board\n"));
    assert!(restored_note.contains("tags:\n- visual\n"));
    assert!(restored_note.contains("asset: assets/board.png\n"));
    assert!(!restored_note.contains(r#"{"type":"image""#));
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["build", "--no-embed"])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["search", "온톨로지"])
        .assert()
        .success()
        .stdout(predicate::str::contains("board#0000"));
}

#[test]
fn bundle_directory_imports_as_one_portable_artifact() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let bundle = dir.path().join("bundle");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("assets/clip.mp4"), b"mp4 bytes").unwrap();
    std::fs::write(
        source.join("notes/clip.md"),
        "---\ntype: video\ntitle: Clip\nasset: assets/clip.mp4\ntags: [demo]\n---\n번들 복원 가능한 영상 노트.",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args(["bundle", bundle.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("bundle 완료"));

    assert!(bundle.join("context.jsonl").exists());
    assert!(bundle.join("context.md").exists());
    assert!(bundle.join("mcp-context.json").exists());
    assert!(bundle.join("bundle.json").exists());
    assert!(bundle.join("assets/clip.mp4").exists());

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("import 완료: notes=1 assets=1"));

    assert!(restored.join("assets/clip.mp4").exists());
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["build", "--no-embed"])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["search", "영상"])
        .assert()
        .success()
        .stdout(predicate::str::contains("clip#0000"));
}

#[test]
fn bundle_archive_imports_with_same_manifest_contract() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let bundle = dir.path().join("bundle");
    let archive = dir.path().join("bundle.tar.gz");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("assets/clip.mp4"), b"archive mp4 bytes").unwrap();
    std::fs::write(
        source.join("notes/clip.md"),
        "---\ntype: video\ntitle: Clip\nasset: assets/clip.mp4\ntags: [archive]\n---\n압축 번들 복원 가능한 영상 노트.",
    )
    .unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args([
            "bundle",
            bundle.to_str().unwrap(),
            "--archive",
            archive.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("archive 완료"))
        .stdout(predicate::str::contains("bundle 완료"));

    assert!(bundle.join("bundle.json").exists());
    assert!(archive.exists());

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", archive.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("import 완료: notes=1 assets=1"));

    assert_eq!(
        std::fs::read(restored.join("assets/clip.mp4")).unwrap(),
        b"archive mp4 bytes"
    );
    assert!(std::fs::read_to_string(restored.join("notes/clip.md"))
        .unwrap()
        .contains("\ntype: video\n"));
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["build", "--no-embed"])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["search", "압축"])
        .assert()
        .success()
        .stdout(predicate::str::contains("clip#0000"));
}

#[test]
fn bundle_rejects_archive_inside_bundle_directory() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let bundle = dir.path().join("bundle");
    let archive = bundle.join("bundle.tar.gz");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("notes/a.md"), "---\ntitle: A\n---\nportable").unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args([
            "bundle",
            bundle.to_str().unwrap(),
            "--archive",
            archive.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "archive path must be outside bundle directory",
        ));
}

#[test]
fn bundle_import_validates_manifest_and_context_before_restore() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let bundle = dir.path().join("bundle");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("notes/a.md"), "---\ntitle: A\n---\nportable").unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args(["bundle", bundle.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();

    std::fs::remove_file(bundle.join("bundle.json")).unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle manifest missing"));

    std::fs::write(
        bundle.join("bundle.json"),
        r#"{"type":"ontopack.bundle","version":1,"context":"context.jsonl","assets":"assets","notes":1,"assets_copied":0}"#,
    )
    .unwrap();
    std::fs::remove_file(bundle.join("context.jsonl")).unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle context missing"));
}

#[test]
fn bundle_import_rejects_invalid_manifest_identity_and_missing_listed_companions() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let bundle = dir.path().join("bundle");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("notes/a.md"), "---\ntitle: A\n---\nportable").unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args(["bundle", bundle.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();

    std::fs::write(
        bundle.join("bundle.json"),
        r#"{"type":"not.ontopack","version":1,"context":"context.jsonl","markdown":"context.md","mcp_context":"mcp-context.json","assets":"assets","notes":1,"assets_copied":0}"#,
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle manifest invalid type"));

    std::fs::write(
        bundle.join("bundle.json"),
        r#"{"type":"ontopack.bundle","version":999,"context":"context.jsonl","markdown":"context.md","mcp_context":"mcp-context.json","assets":"assets","notes":1,"assets_copied":0}"#,
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "bundle manifest unsupported version",
        ));

    std::fs::write(
        bundle.join("bundle.json"),
        r#"{"type":"ontopack.bundle","version":1,"context":"context.jsonl","markdown":"context.md","mcp_context":"mcp-context.json","assets":"assets","notes":1,"assets_copied":0}"#,
    )
    .unwrap();
    std::fs::remove_file(bundle.join("context.md")).unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle markdown missing"));

    std::fs::write(bundle.join("context.md"), "# restored").unwrap();
    std::fs::remove_file(bundle.join("mcp-context.json")).unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle mcp_context missing"));
}

#[test]
fn bundle_import_fails_when_referenced_asset_is_missing() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let bundle = dir.path().join("bundle");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("assets/clip.mp4"), b"mp4 bytes").unwrap();
    std::fs::write(
        source.join("notes/clip.md"),
        "---\ntype: video\ntitle: Clip\nasset: assets/clip.mp4\n---\nportable clip.",
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args(["bundle", bundle.to_str().unwrap()])
        .assert()
        .success();
    std::fs::remove_file(bundle.join("assets/clip.mp4")).unwrap();

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "import asset missing: assets/clip.mp4",
        ));

    assert!(!restored.join("notes/clip.md").exists());
}

#[test]
fn bundle_import_rejects_manifest_count_mismatches() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let bundle = dir.path().join("bundle");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("assets/clip.mp4"), b"mp4 bytes").unwrap();
    std::fs::write(
        source.join("notes/clip.md"),
        "---\ntype: video\ntitle: Clip\nasset: assets/clip.mp4\n---\nportable clip.",
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args(["bundle", bundle.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();

    std::fs::write(
        bundle.join("bundle.json"),
        r#"{"type":"ontopack.bundle","version":1,"context":"context.jsonl","markdown":"context.md","mcp_context":"mcp-context.json","assets":"assets","notes":2,"assets_copied":1}"#,
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle manifest notes mismatch"));

    std::fs::write(
        bundle.join("bundle.json"),
        r#"{"type":"ontopack.bundle","version":1,"context":"context.jsonl","markdown":"context.md","mcp_context":"mcp-context.json","assets":"assets","notes":1,"assets_copied":2}"#,
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle manifest assets mismatch"));
}

#[test]
fn import_refuses_existing_note_or_asset_unless_overwrite_is_set() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source");
    let restored = dir.path().join("restored");
    let bundle = dir.path().join("bundle");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", source.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(source.join("assets/clip.mp4"), b"new mp4 bytes").unwrap();
    std::fs::write(
        source.join("notes/clip.md"),
        "---\ntype: video\ntitle: Clip\nasset: assets/clip.mp4\n---\nnew portable clip.",
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&source)
        .args(["bundle", bundle.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(restored.join("notes/clip.md"), "old note").unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("import note already exists: clip"));

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap(), "--overwrite"])
        .assert()
        .success()
        .stdout(predicate::str::contains("import 완료: notes=1 assets=1"));
    assert!(std::fs::read_to_string(restored.join("notes/clip.md"))
        .unwrap()
        .contains("new portable clip."));

    std::fs::remove_file(restored.join("notes/clip.md")).unwrap();
    std::fs::write(restored.join("assets/clip.mp4"), b"old bytes").unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "import asset already exists: assets/clip.mp4",
        ));
    assert!(!restored.join("notes/clip.md").exists());

    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap(), "--overwrite"])
        .assert()
        .success();
    assert_eq!(
        std::fs::read(restored.join("assets/clip.mp4")).unwrap(),
        b"new mp4 bytes"
    );
}

#[test]
fn import_rejects_context_and_manifest_path_traversal() {
    let dir = tempdir().unwrap();
    let restored = dir.path().join("restored");
    let context = dir.path().join("context.jsonl");
    let bundle = dir.path().join("bundle");

    Command::cargo_bin("pack")
        .unwrap()
        .args(["init", restored.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(
        &context,
        r#"{"note_id":"../evil","body":"bad","asset_path":null}"#,
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", context.to_str().unwrap(), "--format", "jsonl"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsafe import note id"));

    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("context.jsonl"), "").unwrap();
    std::fs::write(
        bundle.join("bundle.json"),
        r#"{"type":"ontopack.bundle","version":1,"context":"../context.jsonl","assets":"assets","notes":0,"assets_copied":0}"#,
    )
    .unwrap();
    Command::cargo_bin("pack")
        .unwrap()
        .current_dir(&restored)
        .args(["import", bundle.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsafe bundle manifest path"));
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
