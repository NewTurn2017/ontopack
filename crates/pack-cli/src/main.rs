#[cfg(not(feature = "real-embed"))]
use anyhow::bail;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use pack_core::enrichment::EnrichmentPatch;
use pack_core::pack::{find_pack_root, AddOutcome, Pack, PackObject, PackStatus};
use pack_core::search::{RankSource, SearchHit};
use serde_json::json;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

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
        Commands::Export { format, output } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let body = export_pack(&pack, format)?;
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
        }
        Commands::Serve {
            port,
            once,
            request,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            let listener = pack_server::http::bind_localhost(port)?;
            let url = pack_server::http::listener_url(&listener)?;
            println!("뷰어 서버: {url}");
            if once {
                if let Some(request) = request {
                    let response = pack_server::http::handle_request(&pack, &request)?;
                    println!("{}", String::from_utf8_lossy(&response.body));
                } else {
                    pack_server::http::serve_once(&pack, &listener)?;
                }
            } else {
                pack_server::http::serve_forever(pack, listener)?;
            }
        }
        Commands::Open {
            port,
            no_browser,
            print_url,
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
            open_browser(&url)?;
            pack_server::http::serve_forever(pack, listener)?;
        }
    }
    Ok(())
}

struct EnrichPendingReport {
    processed: usize,
    skipped: usize,
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
