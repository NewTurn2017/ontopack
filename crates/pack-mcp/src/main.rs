use anyhow::{Context, Result};
use clap::Parser;
use pack_core::pack::find_pack_root;
use pack_mcp::server::{handle_json_line, McpServer};
use std::io::{BufRead, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pack-mcp", about = "ontopack MCP stdio server")]
struct Args {
    /// Pack root containing pack.toml. Defaults to searching from the current directory.
    #[arg(long)]
    pack_root: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = match args.pack_root {
        Some(root) => root,
        None => find_pack_root(&std::env::current_dir()?)?,
    };
    let server = McpServer::open(&root)
        .with_context(|| format!("failed to open pack root {}", root.display()))?;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_json_line(&server, &line)? {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
