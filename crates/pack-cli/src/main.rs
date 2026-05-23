#[cfg(not(feature = "real-embed"))]
use anyhow::bail;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use pack_core::pack::{find_pack_root, AddOutcome, Pack};
use pack_core::search::{RankSource, SearchHit};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

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
        Commands::Build {
            incremental,
            no_embed: _,
        } => {
            let root = find_pack_root(&std::env::current_dir()?)?;
            let pack = Pack::open(&root)?;
            if incremental {
                let report = pack.build_index_incremental()?;
                println!(
                    "증분 인덱스 빌드 완료: indexed={} skipped={} removed={}",
                    report.indexed, report.skipped, report.removed
                );
            } else {
                let count = pack.build_index()?;
                println!("인덱스 빌드 완료: 노트 {count}개");
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
