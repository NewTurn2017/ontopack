#[cfg(not(feature = "real-embed"))]
use anyhow::bail;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use pack_core::enrichment::EnrichmentPatch;
use pack_core::pack::{
    find_pack_root, AddOutcome, DuplicateGroup, LinkGap, OrphanNote, Pack, PackObject, PackStatus,
    RelatedSuggestion, TopicMap,
};
use pack_core::search::{RankSource, SearchHit};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "pack", about = "ontopack — 로컬 지식 팩 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 새 팩 골격을 만든다
    Init {
        /// 팩 경로 (기본: 현재 디렉터리)
        path: Option<PathBuf>,
    },
    /// 파일을 팩에 추가한다 (md→notes/, 그 외→assets/+사이드카)
    Add {
        /// 추가할 파일 경로
        file: PathBuf,
        /// 개체 타입 (기본: note)
        #[arg(long, default_value = "note")]
        r#type: String,
    },
    /// _inbox 파일을 notes/assets로 정리한다
    Process,
    /// 팩 저장/인덱스/enrichment 상태를 요약한다
    Status {
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// 팩 객체 목록을 출력한다
    List {
        /// AI enrichment가 필요한 객체만 출력한다
        #[arg(long)]
        pending_enrichment: bool,
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// 본문이 같은 중복 후보 노트 그룹을 찾는다
    Duplicates {
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// incoming/outgoing 링크가 없는 외톨이 노트를 찾는다
    Orphans {
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// 존재하지 않는 노트로 향하는 깨진 wiki link를 찾는다
    Gaps {
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// 태그 기반 토픽맵을 생성한다
    Topics {
        /// 최소 등장 노트 수
        #[arg(long, default_value_t = 1)]
        min_count: usize,
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// 태그가 겹치지만 아직 연결되지 않은 관련 노트를 추천한다
    Recommend {
        /// 특정 source note id만 추천한다
        note_id: Option<String>,
        /// source note별 최대 추천 수
        #[arg(short, default_value_t = 10)]
        k: usize,
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// sidecar note에 안전한 AI enrichment 섹션을 쓴다
    EnrichNote {
        /// 업데이트할 note id
        note_id: String,
        /// AI caption 텍스트
        #[arg(long)]
        caption: Option<String>,
        /// AI/Ontology tag. 여러 번 지정 가능
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// transcript 텍스트 파일 경로
        #[arg(long)]
        transcript: Option<PathBuf>,
        /// enrichment provider 이름
        #[arg(long)]
        provider: Option<String>,
        /// enrichment model 이름
        #[arg(long)]
        model: Option<String>,
    },
    /// pending media sidecar를 외부 provider command로 자동 enrichment한다
    EnrichPending {
        /// JSON stdin을 받아 EnrichmentPatch JSON stdout을 반환하는 provider 실행 파일
        #[arg(long)]
        provider_command: PathBuf,
        /// provider command에 추가로 전달할 인자. 여러 번 지정 가능
        #[arg(long = "provider-arg")]
        provider_args: Vec<String>,
        /// 처리할 최대 pending 객체 수
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// 처리 후 검색 인덱스 재빌드를 건너뛴다
        #[arg(long)]
        no_rebuild: bool,
    },
    /// 인덱스를 (재)빌드한다
    Build {
        /// 변경된 노트만 갱신한다
        #[arg(long)]
        incremental: bool,
        /// 임베딩 없이 키워드/청크 인덱스만 빌드한다
        #[arg(long)]
        no_embed: bool,
    },
    /// _inbox 처리와 증분 인덱싱을 반복 실행한다
    Watch {
        /// 한 번만 실행하고 종료한다
        #[arg(long)]
        once: bool,
        /// 폴링 간격(ms)
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
    },
    /// 설치/팩 상태를 진단한다
    Doctor {
        /// JSON으로 출력한다
        #[arg(long)]
        json: bool,
    },
    /// 실제 임베딩 모델로 chunks 벡터 인덱스를 빌드한다
    Embed {
        /// 키워드/청크 인덱스 재빌드를 건너뛴다
        #[arg(long)]
        skip_build: bool,
    },
    /// 키워드 검색
    Search {
        /// 검색어
        query: String,
        /// 최대 결과 수
        #[arg(short, default_value_t = 10)]
        k: usize,
        /// 검색 모드
        #[arg(long, value_enum, default_value_t = SearchModeArg::Keyword)]
        mode: SearchModeArg,
    },
    /// 팩 내용을 외부 도구/LLM/강의 번들용 portable context로 내보낸다
    Export {
        /// 출력 형식
        #[arg(long, value_enum, default_value_t = ExportFormatArg::MarkdownBundle)]
        format: ExportFormatArg,
        /// 출력 파일 경로. 생략하면 stdout으로 출력한다
        #[arg(long)]
        output: Option<PathBuf>,
        /// portable bundle용 asset 복사 디렉터리. asset path를 보존해 <dir>/assets/...로 복사한다
        #[arg(long)]
        copy_assets: Option<PathBuf>,
    },
    /// portable JSONL context를 현재 팩으로 불러온다
    Import {
        /// `pack export --format jsonl`로 만든 파일
        input: PathBuf,
        /// 입력 형식
        #[arg(long, value_enum, default_value_t = ImportFormatArg::Jsonl)]
        format: ImportFormatArg,
        /// `pack export --copy-assets`로 만든 asset root 디렉터리
        #[arg(long)]
        asset_root: Option<PathBuf>,
        /// 기존 note/asset 파일이 있으면 덮어쓴다
        #[arg(long)]
        overwrite: bool,
    },
    /// context + assets를 한 디렉터리 bundle artifact로 묶는다
    Bundle {
        /// bundle 출력 디렉터리
        output: PathBuf,
        /// bundle 디렉터리 레이아웃을 gzip-compressed tar archive로도 저장한다
        #[arg(long)]
        archive: Option<PathBuf>,
    },
    /// 로컬 HTTP 뷰어/API 서버를 시작한다
    Serve {
        /// 바인딩할 로컬 포트 (0이면 임의 포트)
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// 테스트/스모크용: 요청 하나만 처리하고 종료
        #[arg(long)]
        once: bool,
        /// --once에서 사용할 raw HTTP 요청
        #[arg(long)]
        request: Option<String>,
        /// real-embed 빌드에서 서버 프로세스에 임베더를 한 번 로드해 vector/hybrid 검색을 활성화한다
        #[arg(long)]
        semantic: bool,
    },
    /// 로컬 뷰어를 브라우저로 연다
    Open {
        /// 바인딩할 로컬 포트 (0이면 임의 포트)
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// 브라우저를 실행하지 않는다
        #[arg(long)]
        no_browser: bool,
        /// URL을 stdout에 출력한다
        #[arg(long)]
        print_url: bool,
        /// real-embed 빌드에서 서버 프로세스에 임베더를 한 번 로드해 vector/hybrid 검색을 활성화한다
        #[arg(long)]
        semantic: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SearchModeArg {
    Keyword,
    Vector,
    Hybrid,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ExportFormatArg {
    MarkdownBundle,
    Jsonl,
    McpContext,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ImportFormatArg {
    Jsonl,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path } => {
            let root = path.unwrap_or_else(|| PathBuf::from("."));
            let name = root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("my-pack")
                .to_string();
            Pack::init(&root, &name)?;
            println!("팩 초기화 완료: {}", root.display());
        }
        Commands::Add { file, r#type } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            match pack.add_file(&file, &r#type)? {
                AddOutcome::Note { path } => println!("노트 추가: {}", path.display()),
                AddOutcome::AssetWithSidecar { note_path, .. } => {
                    println!("자산+사이드카 추가: {}", note_path.display());
                }
            }
        }
        Commands::Process => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let report = pack.process_inbox()?;
            println!("인박스 처리 완료: {}개", report.processed);
        }
        Commands::Status { json } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let status = pack.status()?;
            let manifest = pack.refresh_object_manifest()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                print_status(&status, &manifest);
            }
        }
        Commands::List {
            pending_enrichment,
            json,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let objects = if pending_enrichment {
                pack.pending_enrichment_objects()?
            } else {
                pack.objects()?
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&objects)?);
            } else {
                print_objects(&objects);
            }
        }
        Commands::Duplicates { json } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let groups = pack.duplicate_notes()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&groups)?);
            } else {
                print_duplicate_groups(&groups);
            }
        }
        Commands::Orphans { json } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let orphans = pack.orphan_notes()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&orphans)?);
            } else {
                print_orphan_notes(&orphans);
            }
        }
        Commands::Gaps { json } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let gaps = pack.link_gaps()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&gaps)?);
            } else {
                print_link_gaps(&gaps);
            }
        }
        Commands::Topics { min_count, json } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let topic_map = pack.topic_map(min_count)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&topic_map)?);
            } else {
                print_topic_map(&topic_map);
            }
        }
        Commands::Recommend { note_id, k, json } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let suggestions = pack.related_suggestions(note_id.as_deref(), k)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&suggestions)?);
            } else {
                print_related_suggestions(&suggestions);
            }
        }
        Commands::EnrichNote {
            note_id,
            caption,
            tags,
            transcript,
            provider,
            model,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let transcript = match transcript {
                Some(path) => Some(std::fs::read_to_string(path)?),
                None => None,
            };
            let patch = EnrichmentPatch {
                caption,
                tags,
                transcript,
                provider,
                model,
                ..EnrichmentPatch::default()
            };
            let path = pack.update_enrichment(&note_id, &patch)?;
            println!("enrichment 업데이트: {}", path.display());
        }
        Commands::EnrichPending {
            provider_command,
            provider_args,
            limit,
            no_rebuild,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let report =
                enrich_pending_with_command(&pack, &provider_command, &provider_args, limit)?;
            let indexed = if !no_rebuild && report.processed > 0 {
                Some(pack.build_index()?)
            } else {
                None
            };
            let manifest = pack.refresh_object_manifest()?;
            println!(
                "enrichment worker 완료: processed={} skipped={} indexed={} manifest={}",
                report.processed,
                report.skipped,
                indexed
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                manifest.display()
            );
        }
        Commands::Build {
            incremental,
            no_embed,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            if incremental {
                let report = pack.build_index_incremental()?;
                println!(
                    "증분 인덱스 빌드 완료: indexed={} skipped={} removed={}{}",
                    report.indexed,
                    report.skipped,
                    report.removed,
                    if no_embed { " (no-embed)" } else { "" }
                );
            } else {
                let count = pack.build_index()?;
                if no_embed {
                    println!("인덱스 빌드 완료: 노트 {count}개 (no-embed)");
                } else {
                    println!("인덱스 빌드 완료: 노트 {count}개");
                }
            }
        }
        Commands::Watch { once, interval_ms } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let mut cycle = 0usize;
            loop {
                cycle += 1;
                let processed = pack.process_inbox()?.processed;
                let report = pack.build_index_incremental()?;
                let manifest = pack.refresh_object_manifest()?;
                println!(
                    "watch tick: cycle={} processed={} indexed={} skipped={} removed={} manifest={}",
                    cycle,
                    processed,
                    report.indexed,
                    report.skipped,
                    report.removed,
                    manifest.display()
                );
                if once {
                    break;
                }
                std::thread::sleep(Duration::from_millis(interval_ms.max(100)));
            }
        }
        Commands::Doctor { json } => {
            let report = doctor_report()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_doctor_report(&report);
            }
        }
        Commands::Embed { skip_build } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            embed_pack(&pack, skip_build)?;
        }
        Commands::Search { query, k, mode } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let hits = search_pack(&pack, &query, k, mode)?;
            if hits.is_empty() {
                println!("(결과 없음)");
            }
            for h in hits {
                print_search_hit(&h);
            }
        }
        Commands::Export {
            format,
            output,
            copy_assets,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let body = export_pack(&pack, format)?;
            let wrote_to_file = output.is_some();
            if let Some(output) = output {
                if let Some(parent) = output
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&output, body)?;
                println!("export 완료: {}", output.display());
            } else {
                print!("{body}");
            }
            if let Some(copy_assets) = copy_assets {
                let copied = copy_referenced_assets(&pack, &copy_assets)?;
                let message = format!("assets copied={copied} -> {}", copy_assets.display());
                if wrote_to_file {
                    println!("{message}");
                } else {
                    eprintln!("{message}");
                }
            }
        }
        Commands::Import {
            input,
            format,
            asset_root,
            overwrite,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let report =
                import_pack_context(&root, &input, format, asset_root.as_deref(), overwrite)?;
            println!(
                "import 완료: notes={} assets={}",
                report.notes, report.assets
            );
        }
        Commands::Bundle { output, archive } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let report = bundle_pack(&pack, &output)?;
            if let Some(archive) = archive {
                write_bundle_archive(&output, &archive)?;
                println!("archive 완료: {}", archive.display());
            }
            println!(
                "bundle 완료: notes={} assets={} dir={}",
                report.notes,
                report.assets,
                output.display()
            );
        }
        Commands::Serve {
            port,
            once,
            request,
            semantic,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let listener = pack_server::http::bind_localhost(port)?;
            let url = pack_server::http::listener_url(&listener)?;
            println!("뷰어 서버: {url}");
            let state = server_state(pack, semantic)?;
            if once {
                if let Some(request) = request {
                    let response = pack_server::http::handle_request_with_state(&state, &request)?;
                    println!("{}", String::from_utf8_lossy(&response.body));
                } else {
                    pack_server::http::serve_once_with_state(&state, &listener)?;
                }
            } else {
                pack_server::http::serve_forever_with_state(state, listener)?;
            }
        }
        Commands::Open {
            port,
            no_browser,
            print_url,
            semantic,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let listener = pack_server::http::bind_localhost(port)?;
            let url = pack_server::http::listener_url(&listener)?;
            if print_url || no_browser {
                println!("{url}");
            }
            if no_browser {
                return Ok(());
            }
            let state = server_state(pack, semantic)?;
            open_browser(&url)?;
            pack_server::http::serve_forever_with_state(state, listener)?;
        }
    }
    Ok(())
}

#[cfg(feature = "real-embed")]
fn server_state(pack: Pack, semantic: bool) -> Result<pack_server::http::ServerState> {
    if semantic {
        pack_server::http::ServerState::with_real_embedder(pack, true)
    } else {
        Ok(pack_server::http::ServerState::new(pack))
    }
}

#[cfg(not(feature = "real-embed"))]
fn server_state(pack: Pack, semantic: bool) -> Result<pack_server::http::ServerState> {
    if semantic {
        bail!(
            "pack serve/open --semantic은 real-embed feature로 빌드해야 합니다: cargo build --release --features real-embed"
        );
    }
    Ok(pack_server::http::ServerState::new(pack))
}

struct EnrichPendingReport {
    processed: usize,
    skipped: usize,
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    ok: bool,
    executable: String,
    cwd: String,
    pack_root: Option<String>,
    checks: Vec<DoctorCheck>,
}

fn doctor_report() -> Result<DoctorReport> {
    let cwd = std::env::current_dir()?;
    let executable = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let pack_root = find_pack_root(&cwd).ok();
    let mut checks = Vec::new();
    checks.push(DoctorCheck {
        name: "executable".to_string(),
        ok: executable != "unknown",
        detail: executable.clone(),
    });
    match &pack_root {
        Some(root) => {
            checks.push(DoctorCheck {
                name: "pack_root".to_string(),
                ok: true,
                detail: root.display().to_string(),
            });
            for rel in ["pack.toml", "notes", "assets", "_inbox", ".pack"] {
                let path = root.join(rel);
                checks.push(DoctorCheck {
                    name: rel.to_string(),
                    ok: path.exists(),
                    detail: path.display().to_string(),
                });
            }
            let index_path = root.join(".pack/index.db");
            checks.push(DoctorCheck {
                name: "index".to_string(),
                ok: index_path.exists(),
                detail: if index_path.exists() {
                    index_path.display().to_string()
                } else {
                    format!("{} (run: pack build --incremental)", index_path.display())
                },
            });
        }
        None => checks.push(DoctorCheck {
            name: "pack_root".to_string(),
            ok: false,
            detail: "pack.toml not found from current directory upward".to_string(),
        }),
    }
    let ok = checks.iter().all(|check| check.ok);
    Ok(DoctorReport {
        ok,
        executable,
        cwd: cwd.display().to_string(),
        pack_root: pack_root.map(|root| root.display().to_string()),
        checks,
    })
}

fn print_doctor_report(report: &DoctorReport) {
    println!("doctor: ok={}", report.ok);
    println!("executable={}", report.executable);
    println!("cwd={}", report.cwd);
    println!("pack_root={}", report.pack_root.as_deref().unwrap_or("-"));
    for check in &report.checks {
        println!(
            "- {} {} {}",
            if check.ok { "ok" } else { "fail" },
            check.name,
            check.detail
        );
    }
}

fn enrich_pending_with_command(
    pack: &Pack,
    provider_command: &std::path::Path,
    provider_args: &[String],
    limit: usize,
) -> Result<EnrichPendingReport> {
    let mut pending = pack.pending_enrichment_objects()?;
    pending.truncate(limit);
    let notes = pack.scan_notes()?;
    let mut by_id = std::collections::HashMap::new();
    for note in notes {
        by_id.insert(note.id.clone(), note);
    }

    let mut processed = 0usize;
    let mut skipped = 0usize;
    for object in pending {
        let Some(note) = by_id.get(&object.note_id) else {
            skipped += 1;
            continue;
        };
        let raw = std::fs::read_to_string(&note.path)?;
        let asset_abs_path = note
            .asset
            .as_ref()
            .map(|asset| pack.root.join(asset).to_string_lossy().to_string());
        let payload = json!({
            "note_id": note.id,
            "title": note.title,
            "note_type": note.note_type,
            "tags": note.tags,
            "created": note.created,
            "related": note.related,
            "note_path": note.path.to_string_lossy(),
            "asset_path": note.asset,
            "asset_abs_path": asset_abs_path,
            "body": note.body,
            "raw": raw,
            "content_hash": object.content_hash
        });
        let patch = run_provider_command(provider_command, provider_args, &payload)?;
        pack.update_enrichment(&note.id, &patch)?;
        processed += 1;
    }

    Ok(EnrichPendingReport { processed, skipped })
}

fn run_provider_command(
    provider_command: &std::path::Path,
    provider_args: &[String],
    payload: &serde_json::Value,
) -> Result<EnrichmentPatch> {
    let mut child = ProcessCommand::new(provider_command)
        .args(provider_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("provider stdin을 열 수 없습니다"))?;
        serde_json::to_writer(&mut *stdin, payload)?;
        stdin.write_all(b"\n")?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!(
            "provider command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn print_search_hit(hit: &SearchHit) {
    println!(
        "[{}] {}  ({} / {}) {}",
        rank_source_label(hit.rank_source),
        hit.title,
        hit.note_id,
        hit.chunk_id,
        hit.snippet.replace('\n', " ")
    );
}

fn print_status(status: &PackStatus, manifest: &std::path::Path) {
    println!(
        "팩 상태: total={} notes={} assets={} indexed={} pending_enrichment={} done_enrichment={} error_enrichment={}",
        status.total,
        status.notes,
        status.assets,
        status.indexed,
        status.pending_enrichment,
        status.done_enrichment,
        status.error_enrichment
    );
    println!("객체 manifest 갱신: {}", manifest.display());
}

fn print_duplicate_groups(groups: &[DuplicateGroup]) {
    if groups.is_empty() {
        println!("중복 후보 없음");
        return;
    }
    println!("중복 후보: groups={}", groups.len());
    for group in groups {
        println!(
            "fingerprint={} count={} normalized_len={}",
            group.fingerprint,
            group.candidates.len(),
            group.normalized_len
        );
        for candidate in &group.candidates {
            println!(
                "- {} [{}] title={} path={} asset={}",
                candidate.note_id,
                candidate.note_type,
                candidate.title,
                candidate.path,
                candidate.asset.as_deref().unwrap_or("-")
            );
        }
    }
}

fn print_orphan_notes(orphans: &[OrphanNote]) {
    if orphans.is_empty() {
        println!("외톨이 노트 없음");
        return;
    }
    println!("외톨이 노트: count={}", orphans.len());
    for note in orphans {
        println!(
            "- {} [{}] title={} path={} asset={}",
            note.note_id,
            note.note_type,
            note.title,
            note.path,
            note.asset.as_deref().unwrap_or("-")
        );
    }
}

fn print_link_gaps(gaps: &[LinkGap]) {
    if gaps.is_empty() {
        println!("깨진 링크 없음");
        return;
    }
    println!("깨진 링크: count={}", gaps.len());
    for gap in gaps {
        println!(
            "- {} -> {} title={} path={}",
            gap.source_id, gap.missing_target, gap.source_title, gap.source_path
        );
    }
}

fn print_topic_map(topic_map: &TopicMap) {
    println!(
        "토픽맵: topics={} edges={}",
        topic_map.topics.len(),
        topic_map.edges.len()
    );
    for topic in &topic_map.topics {
        println!(
            "- topic {} count={} notes={}",
            topic.topic,
            topic.note_count,
            topic.notes.join(",")
        );
    }
    for edge in &topic_map.edges {
        println!(
            "- edge {} -- {} weight={} notes={}",
            edge.source,
            edge.target,
            edge.weight,
            edge.notes.join(",")
        );
    }
}

fn print_related_suggestions(suggestions: &[RelatedSuggestion]) {
    if suggestions.is_empty() {
        println!("관련 노트 추천 없음");
        return;
    }
    println!("관련 노트 추천: count={}", suggestions.len());
    for item in suggestions {
        println!(
            "- {} -> {} score={} tags={} source_title={} candidate_title={}",
            item.source_id,
            item.candidate_id,
            item.score,
            item.shared_tags.join(","),
            item.source_title,
            item.candidate_title
        );
    }
}

fn print_objects(objects: &[PackObject]) {
    if objects.is_empty() {
        println!("(객체 없음)");
    }
    for object in objects {
        println!(
            "{} [{}] kind={} indexed={} enrichment={:?} asset={}",
            object.note_id,
            object.note_type,
            object.kind,
            object.indexed,
            object.enrichment_status,
            object.asset_path.as_deref().unwrap_or("-")
        );
    }
}

fn export_pack(pack: &Pack, format: ExportFormatArg) -> Result<String> {
    let mut notes = pack.scan_notes()?;
    notes.sort_by(|a, b| a.id.cmp(&b.id));
    match format {
        ExportFormatArg::MarkdownBundle => export_markdown_bundle(pack, &notes),
        ExportFormatArg::Jsonl => export_jsonl(pack, &notes),
        ExportFormatArg::McpContext => export_mcp_context(pack, &notes),
    }
}

fn copy_referenced_assets(pack: &Pack, destination: &Path) -> Result<usize> {
    let mut assets = std::collections::BTreeSet::new();
    for note in pack.scan_notes()? {
        if let Some(asset) = note.asset {
            assets.insert(asset);
        }
        for asset in extract_asset_paths(&note.body) {
            assets.insert(asset);
        }
    }

    let mut copied = 0usize;
    for asset in assets {
        ensure_safe_asset_path(&asset)?;
        let source = pack.root.join(&asset);
        if !source.is_file() {
            anyhow::bail!("export asset missing: {asset}");
        }
        let target = destination.join(&asset);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&source, &target)?;
        copied += 1;
    }
    Ok(copied)
}

fn bundle_pack(pack: &Pack, output: &Path) -> Result<ImportReport> {
    std::fs::create_dir_all(output)?;
    std::fs::write(
        output.join("context.jsonl"),
        export_pack(pack, ExportFormatArg::Jsonl)?,
    )?;
    std::fs::write(
        output.join("context.md"),
        export_pack(pack, ExportFormatArg::MarkdownBundle)?,
    )?;
    std::fs::write(
        output.join("mcp-context.json"),
        export_pack(pack, ExportFormatArg::McpContext)?,
    )?;
    let assets = copy_referenced_assets(pack, output)?;
    let notes = pack.scan_notes()?.len();
    let manifest = json!({
        "type": "ontopack.bundle",
        "version": 1,
        "context": "context.jsonl",
        "markdown": "context.md",
        "mcp_context": "mcp-context.json",
        "assets": "assets",
        "notes": notes,
        "assets_copied": assets,
    });
    std::fs::write(
        output.join("bundle.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(ImportReport { notes, assets })
}

fn extract_asset_paths(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in body.split_whitespace() {
        let token = token.trim_matches(|c: char| {
            matches!(
                c,
                '`' | '"' | '\'' | '(' | ')' | '[' | ']' | '<' | '>' | ',' | ';' | ':' | '.'
            )
        });
        if token.starts_with("assets/") && !out.iter().any(|existing| existing == token) {
            out.push(token.to_string());
        }
    }
    out
}

fn ensure_safe_asset_path(asset: &str) -> Result<()> {
    let path = Path::new(asset);
    if !asset.starts_with("assets/") || path.is_absolute() {
        anyhow::bail!("unsafe export asset path: {asset}");
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => anyhow::bail!("unsafe export asset path: {asset}"),
        }
    }
    Ok(())
}

struct ImportReport {
    notes: usize,
    assets: usize,
}

#[derive(Deserialize)]
struct BundleManifest {
    #[serde(rename = "type")]
    bundle_type: String,
    version: u64,
    context: String,
    markdown: Option<String>,
    mcp_context: Option<String>,
    assets: String,
    notes: usize,
    assets_copied: usize,
}

struct ImportPlan {
    entries: Vec<ImportEntry>,
    assets: std::collections::BTreeSet<String>,
}

struct ImportEntry {
    note_id: String,
    body: String,
    frontmatter: serde_json::Map<String, serde_json::Value>,
}

fn import_pack_context(
    root: &Path,
    input: &Path,
    format: ImportFormatArg,
    asset_root: Option<&Path>,
    overwrite: bool,
) -> Result<ImportReport> {
    match format {
        ImportFormatArg::Jsonl => {
            if input.is_dir() {
                import_bundle_context(root, input, asset_root, overwrite)
            } else if is_bundle_archive(input) {
                if asset_root.is_some() {
                    anyhow::bail!("archive import does not support --asset-root");
                }
                import_bundle_archive(root, input, overwrite)
            } else {
                import_jsonl_context(root, input, asset_root, overwrite)
            }
        }
    }
}

fn write_bundle_archive(bundle_dir: &Path, archive_path: &Path) -> Result<()> {
    if let Some(parent) = archive_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    ensure_archive_outside_bundle(bundle_dir, archive_path)?;
    let file = File::create(archive_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    builder.append_dir_all(".", bundle_dir)?;
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn ensure_archive_outside_bundle(bundle_dir: &Path, archive_path: &Path) -> Result<()> {
    let bundle_dir = bundle_dir.canonicalize()?;
    let archive_parent = archive_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    let archive_abs = archive_parent.join(
        archive_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("invalid archive path: {}", archive_path.display()))?,
    );
    if archive_abs.starts_with(&bundle_dir) {
        anyhow::bail!(
            "archive path must be outside bundle directory: {}",
            archive_path.display()
        );
    }
    Ok(())
}

fn import_bundle_archive(
    root: &Path,
    archive_path: &Path,
    overwrite: bool,
) -> Result<ImportReport> {
    let temp = std::env::temp_dir().join(format!(
        "ontopack-bundle-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp)?;
    let result = (|| -> Result<ImportReport> {
        let file = File::open(archive_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_path = entry.path()?.into_owned();
            if !entry.unpack_in(&temp)? {
                anyhow::bail!("unsafe archive entry path: {}", entry_path.display());
            }
        }
        import_bundle_context(root, &temp, None, overwrite)
    })();
    let cleanup = std::fs::remove_dir_all(&temp);
    match (result, cleanup) {
        (Ok(report), Ok(())) => Ok(report),
        (Ok(report), Err(err)) => {
            eprintln!("warning: archive import cleanup failed: {err}");
            Ok(report)
        }
        (Err(err), _) => Err(err),
    }
}

fn is_bundle_archive(input: &Path) -> bool {
    let Some(name) = input.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    name.ends_with(".tar.gz") || name.ends_with(".tgz")
}

fn import_bundle_context(
    root: &Path,
    bundle: &Path,
    asset_root_override: Option<&Path>,
    overwrite: bool,
) -> Result<ImportReport> {
    let manifest = read_bundle_manifest(bundle)?;
    let context_rel = ensure_safe_bundle_path(&manifest.context, "context")?;
    let assets_rel = ensure_safe_bundle_path(&manifest.assets, "assets")?;

    let context = bundle.join(&context_rel);
    if !context.is_file() {
        anyhow::bail!("bundle context missing: {}", context_rel.display());
    }
    if manifest.assets_copied > 0 && !bundle.join(&assets_rel).is_dir() {
        anyhow::bail!("bundle assets missing: {}", assets_rel.display());
    }
    if let Some(markdown) = &manifest.markdown {
        let markdown_rel = ensure_safe_bundle_path(markdown, "markdown")?;
        if !bundle.join(&markdown_rel).is_file() {
            anyhow::bail!("bundle markdown missing: {}", markdown_rel.display());
        }
    }
    if let Some(mcp_context) = &manifest.mcp_context {
        let mcp_context_rel = ensure_safe_bundle_path(mcp_context, "mcp_context")?;
        if !bundle.join(&mcp_context_rel).is_file() {
            anyhow::bail!("bundle mcp_context missing: {}", mcp_context_rel.display());
        }
    }

    let plan = plan_jsonl_import(&context)?;
    if plan.entries.len() != manifest.notes {
        anyhow::bail!(
            "bundle manifest notes mismatch: expected {} actual {}",
            manifest.notes,
            plan.entries.len()
        );
    }
    if plan.assets.len() != manifest.assets_copied {
        anyhow::bail!(
            "bundle manifest assets mismatch: expected {} actual {}",
            manifest.assets_copied,
            plan.assets.len()
        );
    }

    let default_asset_root = bundle.to_path_buf();
    let asset_root = asset_root_override.unwrap_or(default_asset_root.as_path());
    execute_import_plan(root, plan, Some(asset_root), overwrite)
}

fn read_bundle_manifest(bundle: &Path) -> Result<BundleManifest> {
    let manifest_path = bundle.join("bundle.json");
    if !manifest_path.is_file() {
        anyhow::bail!("bundle manifest missing: {}", manifest_path.display());
    }
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|err| anyhow::anyhow!("bundle manifest unreadable: {err}"))?;
    let manifest: BundleManifest = serde_json::from_str(&raw)
        .map_err(|err| anyhow::anyhow!("bundle manifest invalid: {err}"))?;
    if manifest.bundle_type != "ontopack.bundle" {
        anyhow::bail!("bundle manifest invalid type: {}", manifest.bundle_type);
    }
    if manifest.version != 1 {
        anyhow::bail!("bundle manifest unsupported version: {}", manifest.version);
    }
    Ok(manifest)
}

fn ensure_safe_bundle_path(path: &str, field: &str) -> Result<PathBuf> {
    if path.is_empty() {
        anyhow::bail!("unsafe bundle manifest path for {field}: empty");
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        anyhow::bail!("unsafe bundle manifest path for {field}: {path}");
    }
    for component in candidate.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => anyhow::bail!("unsafe bundle manifest path for {field}: {path}"),
        }
    }
    Ok(candidate.to_path_buf())
}

fn import_jsonl_context(
    root: &Path,
    input: &Path,
    asset_root: Option<&Path>,
    overwrite: bool,
) -> Result<ImportReport> {
    let plan = plan_jsonl_import(input)?;
    execute_import_plan(root, plan, asset_root, overwrite)
}

fn plan_jsonl_import(input: &Path) -> Result<ImportPlan> {
    let raw = std::fs::read_to_string(input)?;
    let mut entries = Vec::new();
    let mut assets = std::collections::BTreeSet::new();
    for (idx, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|err| anyhow::anyhow!("invalid jsonl line {}: {err}", idx + 1))?;
        let note_id = json_string(&value, "note_id")?;
        ensure_safe_note_id(&note_id)?;
        let body = json_string(&value, "body")?;
        let asset_path = json_optional_string(&value, "asset_path")?;
        if let Some(asset) = &asset_path {
            ensure_safe_asset_path(asset)?;
            assets.insert(asset.clone());
        }
        for asset in extract_asset_paths(&body) {
            ensure_safe_asset_path(&asset)?;
            assets.insert(asset);
        }

        let mut frontmatter = serde_json::Map::new();
        frontmatter.insert(
            "type".to_string(),
            json!(json_optional_string(&value, "type")?.unwrap_or_else(|| "note".to_string())),
        );
        frontmatter.insert(
            "title".to_string(),
            json!(json_optional_string(&value, "title")?.unwrap_or_else(|| note_id.clone())),
        );
        if let Some(tags) = json_string_array(&value, "tags")? {
            frontmatter.insert("tags".to_string(), json!(tags));
        }
        if let Some(created) = json_optional_string(&value, "created")? {
            frontmatter.insert("created".to_string(), json!(created));
        }
        if let Some(asset) = asset_path {
            frontmatter.insert("asset".to_string(), json!(asset));
        }
        if let Some(related) = json_string_array(&value, "related")? {
            frontmatter.insert("related".to_string(), json!(related));
        }

        entries.push(ImportEntry {
            note_id,
            body,
            frontmatter,
        });
    }

    Ok(ImportPlan { entries, assets })
}

fn execute_import_plan(
    root: &Path,
    plan: ImportPlan,
    asset_root: Option<&Path>,
    overwrite: bool,
) -> Result<ImportReport> {
    for entry in &plan.entries {
        let note_path = root.join("notes").join(format!("{}.md", entry.note_id));
        if note_path.exists() && !overwrite {
            anyhow::bail!("import note already exists: {}", entry.note_id);
        }
    }

    if let Some(asset_root) = asset_root {
        for asset in &plan.assets {
            let source = asset_root.join(asset);
            if !source.is_file() {
                anyhow::bail!("import asset missing: {asset}");
            }
            let target = root.join(asset);
            if target.exists() && !overwrite {
                anyhow::bail!("import asset already exists: {asset}");
            }
        }
    }

    for entry in &plan.entries {
        let note_path = root.join("notes").join(format!("{}.md", entry.note_id));
        if let Some(parent) = note_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let note = format!(
            "---\n{}---\n{}",
            serde_yaml::to_string(&entry.frontmatter)?,
            entry.body
        );
        std::fs::write(note_path, note)?;
    }

    let mut copied_assets = 0usize;
    if let Some(asset_root) = asset_root {
        for asset in &plan.assets {
            let source = asset_root.join(asset);
            let target = root.join(asset);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(source, target)?;
            copied_assets += 1;
        }
    }

    Ok(ImportReport {
        notes: plan.entries.len(),
        assets: copied_assets,
    })
}

fn ensure_safe_note_id(note_id: &str) -> Result<()> {
    if note_id.is_empty() {
        anyhow::bail!("unsafe import note id: empty");
    }
    let path = Path::new(note_id);
    if path.is_absolute() {
        anyhow::bail!("unsafe import note id: {note_id}");
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => anyhow::bail!("unsafe import note id: {note_id}"),
        }
    }
    Ok(())
}

fn json_string(value: &serde_json::Value, key: &str) -> Result<String> {
    json_optional_string(value, key)?.ok_or_else(|| anyhow::anyhow!("missing string field: {key}"))
}

fn json_optional_string(value: &serde_json::Value, key: &str) -> Result<Option<String>> {
    match value.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
        _ => anyhow::bail!("expected string field: {key}"),
    }
}

fn json_string_array(value: &serde_json::Value, key: &str) -> Result<Option<Vec<String>>> {
    match value.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Array(items)) => {
            let mut out = Vec::new();
            for item in items {
                let Some(s) = item.as_str() else {
                    anyhow::bail!("expected string array field: {key}");
                };
                out.push(s.to_string());
            }
            Ok(Some(out))
        }
        _ => anyhow::bail!("expected string array field: {key}"),
    }
}

fn export_markdown_bundle(pack: &Pack, notes: &[pack_core::note::Note]) -> Result<String> {
    let mut out = String::new();
    out.push_str("# OntoPack Markdown Bundle\n\n");
    out.push_str(&format!("Pack: `{}`\n\n", pack.config.name));
    for note in notes {
        let note_path = display_pack_path(pack, &note.path);
        out.push_str(&format!("## {}\n\n", note.title));
        out.push_str(&format!("- Citation: `note:{}`\n", note.id));
        out.push_str(&format!("- Note ID: `{}`\n", note.id));
        out.push_str(&format!("- Type: `{}`\n", note.note_type));
        if !note.tags.is_empty() {
            out.push_str(&format!("- Tags: `{}`\n", note.tags.join("`, `")));
        }
        if let Some(created) = &note.created {
            out.push_str(&format!("- Created: `{created}`\n"));
        }
        out.push_str(&format!("- Note path: `{note_path}`\n"));
        if let Some(asset) = &note.asset {
            out.push_str(&format!("- Asset: `{asset}`\n"));
        }
        if !note.related.is_empty() {
            out.push_str(&format!("- Related: `{}`\n", note.related.join("`, `")));
        }
        out.push('\n');
        out.push_str(note.body.trim_end());
        out.push_str("\n\n---\n\n");
    }
    Ok(out)
}

fn export_jsonl(pack: &Pack, notes: &[pack_core::note::Note]) -> Result<String> {
    let mut out = String::new();
    for note in notes {
        let note_path = display_pack_path(pack, &note.path);
        let line = json!({
            "note_id": note.id,
            "title": note.title,
            "type": note.note_type,
            "tags": note.tags,
            "created": note.created,
            "note_path": note_path,
            "asset_path": note.asset,
            "related": note.related,
            "body": note.body,
            "citation": {
                "note_id": note.id,
                "note_path": note_path,
                "asset_path": note.asset,
            }
        });
        out.push_str(&serde_json::to_string(&line)?);
        out.push('\n');
    }
    Ok(out)
}

fn export_mcp_context(pack: &Pack, notes: &[pack_core::note::Note]) -> Result<String> {
    let context_blocks: Vec<_> = notes
        .iter()
        .map(|note| {
            let note_path = display_pack_path(pack, &note.path);
            json!({
                "note_id": note.id,
                "title": note.title,
                "type": note.note_type,
                "tags": note.tags,
                "created": note.created,
                "note_path": note_path,
                "asset_path": note.asset,
                "related": note.related,
                "body": note.body,
                "citation": format!("note:{}", note.id),
            })
        })
        .collect();
    let value = json!({
        "type": "ontopack.mcp_context",
        "pack": pack.config.name,
        "instruction": "Use context_blocks as source-grounded local knowledge. Cite note_id or citation for every derived answer.",
        "context_blocks": context_blocks,
    });
    Ok(format!("{}\n", serde_json::to_string(&value)?))
}

fn display_pack_path(pack: &Pack, path: &Path) -> String {
    path.strip_prefix(&pack.root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn rank_source_label(source: RankSource) -> &'static str {
    match source {
        RankSource::Keyword => "keyword",
        RankSource::Vector => "vector",
        RankSource::Hybrid => "hybrid",
    }
}

fn search_pack(pack: &Pack, query: &str, k: usize, mode: SearchModeArg) -> Result<Vec<SearchHit>> {
    match mode {
        SearchModeArg::Keyword => pack.search_keyword_chunks(query, k),
        SearchModeArg::Vector | SearchModeArg::Hybrid => search_pack_semantic(pack, query, k, mode),
    }
}

#[cfg(feature = "real-embed")]
fn search_pack_semantic(
    pack: &Pack,
    query: &str,
    k: usize,
    mode: SearchModeArg,
) -> Result<Vec<SearchHit>> {
    let embedder = pack_core::embed::FastEmbedder::try_new(
        &pack.config.embed_model,
        pack.config.embed_dim,
        true,
    )?;
    match mode {
        SearchModeArg::Keyword => pack.search_keyword_chunks(query, k),
        SearchModeArg::Vector => pack.search_vector_chunk_hits_with(query, k, &embedder),
        SearchModeArg::Hybrid => pack.search_hybrid_with(query, k, &embedder),
    }
}

#[cfg(not(feature = "real-embed"))]
fn search_pack_semantic(
    _pack: &Pack,
    _query: &str,
    _k: usize,
    _mode: SearchModeArg,
) -> Result<Vec<SearchHit>> {
    bail!(
        "vector/hybrid search는 real-embed feature로 빌드해야 합니다: cargo build --release --features real-embed"
    )
}

#[cfg(feature = "real-embed")]
fn embed_pack(pack: &Pack, skip_build: bool) -> Result<()> {
    if !skip_build {
        let count = pack.build_index()?;
        println!("인덱스 빌드 완료: 노트 {count}개");
    }
    let embedder = pack_core::embed::FastEmbedder::try_new(
        &pack.config.embed_model,
        pack.config.embed_dim,
        true,
    )?;
    let count = pack.build_chunk_embeddings_with(&embedder)?;
    println!(
        "임베딩 완료: chunks={} model={} dim={}",
        count, pack.config.embed_model, pack.config.embed_dim
    );
    Ok(())
}

#[cfg(not(feature = "real-embed"))]
fn embed_pack(_pack: &Pack, _skip_build: bool) -> Result<()> {
    bail!(
        "pack embed는 real-embed feature로 빌드해야 합니다: cargo build --release --features real-embed"
    )
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = ProcessCommand::new("open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = ProcessCommand::new("xdg-open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = ProcessCommand::new("cmd");
        command.args(["/C", "start", url]);
        command
    };
    command.spawn()?;
    Ok(())
}
