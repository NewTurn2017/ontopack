#!/usr/bin/env python3
"""Build a visible OntoPack ontology/media pack from public web sources.

The script intentionally downloads thumbnails/previews for visual QA, not original
source assets. Full-size/original asset download is a future explicit opt-in for
`pack ingest ... --download-assets`.
"""
from __future__ import annotations

import argparse
import base64
import html
import json
import os
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

USER_AGENT = "ontopack-visible-ontology-pack/0.1 (+https://github.com/NewTurn2017/ontopack)"
RUN_ID = time.strftime("%Y%m%d-%H%M%S")
BLOCKLIST = {
    "adult", "boobs", "explicit", "gore", "hentai", "naked", "nude", "nsfw",
    "porn", "sex", "sexy", "violence", "xxx",
}
STOP_TAGS = {"civitai", "prompt", "sfw-api", "commons", "internet-archive", "thumbnail"}
LOW_WEIGHT_TOKENS = {"score_9", "score_8_up", "masterpiece", "best quality", "best_quality", "absurdres"}
PROMPT_QUERIES = ["ontology", "knowledge graph", "semantic web", "diagram"]
COMMONS_QUERIES = [
    "ontology graph",
    "knowledge graph",
    "semantic web diagram",
    "wikidata ontology",
    "network diagram",
    "taxonomy diagram",
]
ARCHIVE_QUERIES = [
    'mediatype:(movies) AND (ontology OR "semantic web" OR "knowledge graph" OR "linked data")',
    'mediatype:(movies) AND (data science OR graph OR network OR taxonomy)',
    'mediatype:(movies) AND (education OR lecture) AND (graph OR data OR web)',
]
TRANSPARENT_PNG = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII="
)


@dataclass
class Record:
    kind: str
    source_platform: str
    source_id: str
    title: str
    remote_url: str
    creator: str = ""
    license_status: str = "unknown"
    prompt: str = ""
    description: str = ""
    tags: list[str] | None = None
    thumbnail_url: str = ""
    base_model: str = ""
    seed: str = ""
    sampler: str = ""
    resources: list[str] | None = None
    raw: dict[str, Any] | None = None


def request_json(url: str, timeout: int = 25, retries: int = 3) -> dict[str, Any]:
    last: Exception | None = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=timeout) as response:
                return json.loads(response.read().decode("utf-8"))
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as exc:
            last = exc
            time.sleep(min(2 ** attempt, 4))
    raise RuntimeError(f"failed to fetch JSON after {retries} attempts: {url}: {last}")


def download(url: str, dest: Path, timeout: int = 35, retries: int = 3) -> bool:
    if not url:
        return False
    dest.parent.mkdir(parents=True, exist_ok=True)
    last: Exception | None = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=timeout) as response:
                data = response.read()
            if data:
                dest.write_bytes(data)
                return True
        except (urllib.error.URLError, TimeoutError) as exc:
            last = exc
            time.sleep(min(2 ** attempt, 4))
    print(f"warn: failed to download thumbnail {url}: {last}", file=sys.stderr)
    return False


def text_has_blocked_terms(*values: str) -> bool:
    text = " ".join(v for v in values if v).lower()
    words = set(re.findall(r"[a-zA-Z][a-zA-Z0-9_+-]+", text))
    return bool(words.intersection(BLOCKLIST))


def clean_text(value: Any, fallback: str = "") -> str:
    if value is None:
        return fallback
    text = html.unescape(str(value))
    text = re.sub(r"<[^>]+>", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    return text or fallback


def slug(value: str, fallback: str = "item") -> str:
    value = clean_text(value).lower()
    value = re.sub(r"[^a-z0-9가-힣._-]+", "-", value)
    value = re.sub(r"-+", "-", value).strip("-._")
    return (value or fallback)[:96]


def yaml_scalar(value: str) -> str:
    return json.dumps(value or "", ensure_ascii=False)


def yaml_list(values: Iterable[str]) -> str:
    cleaned = [v for v in dict.fromkeys(values) if v]
    return "[" + ", ".join(yaml_scalar(v) for v in cleaned) + "]"


def safe_tags(*groups: Iterable[str]) -> list[str]:
    out: list[str] = []
    for group in groups:
        for tag in group:
            tag = slug(str(tag), "tag")[:40]
            if tag and tag not in out and not text_has_blocked_terms(tag):
                out.append(tag)
    return out[:16]


def title_from_prompt(prompt: str, fallback: str) -> str:
    prompt = clean_text(prompt, fallback)
    parts = [p.strip() for p in re.split(r"[,.;|]", prompt) if p.strip()]
    for part in parts:
        lowered = part.lower()
        if lowered in LOW_WEIGHT_TOKENS or lowered in STOP_TAGS:
            continue
        if not text_has_blocked_terms(part):
            return part[:80]
    return fallback[:80]


def civitai_prompt_records(limit: int) -> list[Record]:
    out: list[Record] = []
    cursor: str | None = None
    page = 0
    while len(out) < limit and page < 12:
        params = {
            "limit": "100",
            "nsfw": "false",
            "sort": "Most Reactions",
            "period": "Month",
        }
        if cursor:
            params["cursor"] = cursor
        url = "https://civitai.com/api/v1/images?" + urllib.parse.urlencode(params)
        data = request_json(url)
        for item in data.get("items", []):
            meta = item.get("meta") or {}
            prompt = clean_text(meta.get("prompt"))
            if not prompt or item.get("nsfw") or str(item.get("nsfwLevel", "")).lower() not in {"none", "0", ""}:
                continue
            if text_has_blocked_terms(prompt, json.dumps(meta, ensure_ascii=False)):
                continue
            resources = []
            for resource in meta.get("resources") or []:
                name = clean_text(resource.get("name"))
                if name:
                    resources.append(name)
            source_id = str(item.get("id") or f"civitai-{len(out)}")
            out.append(Record(
                kind="prompt",
                source_platform="civitai",
                source_id=source_id,
                title=title_from_prompt(prompt, f"Civitai prompt {source_id}"),
                remote_url=f"https://civitai.com/images/{source_id}",
                creator=clean_text((item.get("user") or {}).get("username")),
                license_status="api-metadata; verify model/resource license before reuse",
                prompt=prompt,
                description=clean_text(meta.get("negativePrompt")),
                tags=safe_tags(["civitai", "prompt", "sfw-api", "generated-image"], resources),
                thumbnail_url="",
                base_model=clean_text(meta.get("Model") or meta.get("model")),
                seed=clean_text(meta.get("seed")),
                sampler=clean_text(meta.get("sampler")),
                resources=resources,
                raw=item,
            ))
            if len(out) >= limit:
                break
        cursor = (data.get("metadata") or {}).get("nextCursor")
        if not cursor:
            break
        page += 1
    return out[:limit]


def commons_image_records(limit: int) -> list[Record]:
    out: list[Record] = []
    seen: set[str] = set()
    for query in COMMONS_QUERIES:
        if len(out) >= limit:
            break
        params = {
            "action": "query",
            "generator": "search",
            "gsrsearch": query,
            "gsrnamespace": "6",
            "gsrlimit": "50",
            "prop": "imageinfo",
            "iiprop": "url|mime|extmetadata",
            "iiurlwidth": "480",
            "format": "json",
            "formatversion": "2",
        }
        data = request_json("https://commons.wikimedia.org/w/api.php?" + urllib.parse.urlencode(params))
        for page in (data.get("query") or {}).get("pages") or []:
            title = clean_text(page.get("title"), "Commons image")
            if title in seen:
                continue
            seen.add(title)
            info = ((page.get("imageinfo") or [{}])[0])
            meta = info.get("extmetadata") or {}
            image_url = info.get("thumburl") or info.get("url") or ""
            license_name = clean_text((meta.get("LicenseShortName") or {}).get("value"), "unknown")
            artist = clean_text((meta.get("Artist") or {}).get("value"))
            desc = clean_text((meta.get("ImageDescription") or {}).get("value"))
            if text_has_blocked_terms(title, desc):
                continue
            source_id = slug(title.replace("File:", ""), "commons-image")
            out.append(Record(
                kind="image",
                source_platform="wikimedia-commons",
                source_id=source_id,
                title=title.replace("File:", ""),
                remote_url="https://commons.wikimedia.org/wiki/" + urllib.parse.quote(title.replace(" ", "_")),
                creator=artist,
                license_status=license_name,
                description=desc,
                tags=safe_tags(["commons", "ontology", "image", "thumbnail"], query.split()),
                thumbnail_url=image_url,
                raw=page,
            ))
            if len(out) >= limit:
                break
    return out[:limit]


def archive_video_records(limit: int) -> list[Record]:
    out: list[Record] = []
    seen: set[str] = set()
    for query in ARCHIVE_QUERIES:
        page_no = 1
        while len(out) < limit and page_no <= 4:
            params = [
                ("q", query),
                ("fl[]", "identifier"),
                ("fl[]", "title"),
                ("fl[]", "creator"),
                ("fl[]", "licenseurl"),
                ("fl[]", "mediatype"),
                ("fl[]", "description"),
                ("fl[]", "subject"),
                ("rows", "50"),
                ("page", str(page_no)),
                ("output", "json"),
            ]
            data = request_json("https://archive.org/advancedsearch.php?" + urllib.parse.urlencode(params))
            docs = (data.get("response") or {}).get("docs") or []
            if not docs:
                break
            for doc in docs:
                ident = clean_text(doc.get("identifier"))
                if not ident or ident in seen:
                    continue
                seen.add(ident)
                title = clean_text(doc.get("title"), ident)
                desc = clean_text(doc.get("description"))
                subjects = doc.get("subject") or []
                if isinstance(subjects, str):
                    subjects = [subjects]
                if text_has_blocked_terms(title, desc, " ".join(map(str, subjects))):
                    continue
                out.append(Record(
                    kind="video",
                    source_platform="internet-archive",
                    source_id=ident,
                    title=title,
                    remote_url=f"https://archive.org/details/{urllib.parse.quote(ident)}",
                    creator=clean_text(doc.get("creator")),
                    license_status=clean_text(doc.get("licenseurl"), "unknown"),
                    description=desc,
                    tags=safe_tags(["internet-archive", "video", "ontology"], map(str, subjects[:10])),
                    thumbnail_url=f"https://archive.org/services/img/{urllib.parse.quote(ident)}",
                    raw=doc,
                ))
                if len(out) >= limit:
                    break
            page_no += 1
    return out[:limit]


def fixture_records(limit: int) -> tuple[list[Record], list[Record], list[Record]]:
    prompts, images, videos = [], [], []
    for i in range(limit):
        prompts.append(Record(
            kind="prompt", source_platform="fixture", source_id=f"prompt-{i:03}",
            title=f"Ontology prompt {i:03}", remote_url=f"https://example.test/prompts/{i:03}",
            creator="fixture", license_status="fixture", prompt=f"ontology graph visual prompt {i:03}",
            tags=["fixture", "prompt", "ontology", f"cluster-{i%10}"], raw={"fixture": True, "i": i},
        ))
        images.append(Record(
            kind="image", source_platform="fixture", source_id=f"image-{i:03}",
            title=f"Ontology image {i:03}", remote_url=f"https://example.test/images/{i:03}",
            creator="fixture", license_status="fixture", description=f"visible ontology image thumbnail {i:03}",
            tags=["fixture", "image", "ontology", f"cluster-{i%10}"], thumbnail_url="fixture://transparent.png",
            raw={"fixture": True, "i": i},
        ))
        videos.append(Record(
            kind="video", source_platform="fixture", source_id=f"video-{i:03}",
            title=f"Ontology video {i:03}", remote_url=f"https://example.test/videos/{i:03}",
            creator="fixture", license_status="fixture", description=f"visible ontology video thumbnail {i:03}",
            tags=["fixture", "video", "ontology", f"cluster-{i%10}"], thumbnail_url="fixture://transparent.png",
            raw={"fixture": True, "i": i},
        ))
    return prompts, images, videos


def preview_mime(record: Record, asset: str | None) -> str:
    if asset:
        clean = asset.lower().split("?", 1)[0]
    else:
        clean = (record.thumbnail_url or record.remote_url).lower().split("?", 1)[0]
    if clean.endswith(".png") or record.thumbnail_url == "fixture://transparent.png":
        return "image/png"
    if clean.endswith(".webp"):
        return "image/webp"
    if clean.endswith(".gif"):
        return "image/gif"
    if clean.endswith((".mp4", ".m4v")) and not record.thumbnail_url:
        return "video/mp4"
    return "image/jpeg" if record.kind in {"image", "video"} else "text/markdown"

def preview_media_kind(record: Record) -> str:
    if record.kind in {"image", "video"} and record.thumbnail_url:
        return "image"
    if record.kind in {"image", "video"}:
        return record.kind
    return "text"


def write_note(path: Path, record: Record, asset: str | None, related: list[str], provenance_path: str) -> None:
    tags = safe_tags(record.tags or [], [record.kind, record.source_platform])
    body = []
    body.append(f"remote_url: {record.remote_url}")
    body.append(f"source_platform: {record.source_platform}")
    body.append(f"source_id: {record.source_id}")
    if record.creator:
        body.append(f"creator: {record.creator}")
    if record.license_status:
        body.append(f"license_status: {record.license_status}")
    if record.base_model:
        body.append(f"base_model: {record.base_model}")
    if record.seed:
        body.append(f"seed: {record.seed}")
    if record.sampler:
        body.append(f"sampler: {record.sampler}")
    if record.resources:
        body.append("resources: " + ", ".join(record.resources[:12]))
    if record.thumbnail_url:
        body.append(f"thumbnail_url: {record.thumbnail_url}")
    body.append(f"provenance: {provenance_path}")
    body.append("")
    if record.prompt:
        body.append("## Prompt")
        body.append(record.prompt)
    if record.description:
        body.append("## Description")
        body.append(record.description)
    body.append("")
    body.append("## Ontology links")
    body.append(" ".join(f"[[{r}]]" for r in related))

    frontmatter = [
        "---",
        f"type: {yaml_scalar(record.kind)}",
        f"title: {yaml_scalar(record.title)}",
        f"tags: {yaml_list(tags)}",
        f"created: {yaml_scalar(time.strftime('%Y-%m-%d'))}",
    ]
    if asset:
        frontmatter.append(f"asset: {yaml_scalar(asset)}")
    if record.remote_url:
        frontmatter.append(f"remote_url: {yaml_scalar(record.remote_url)}")
    if record.thumbnail_url:
        frontmatter.append(f"thumbnail_url: {yaml_scalar(record.thumbnail_url)}")
    if asset or record.thumbnail_url:
        frontmatter.append(f"media_kind: {yaml_scalar(preview_media_kind(record))}")
        frontmatter.append(f"mime: {yaml_scalar(preview_mime(record, asset))}")
    frontmatter.append(f"related: {yaml_list(related)}")
    frontmatter.append("---")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(frontmatter) + "\n" + "\n".join(body).strip() + "\n", encoding="utf-8")


def ensure_platform_notes(pack: Path) -> None:
    notes = {
        "platform-civitai": ("Civitai", "External generated-image prompt metadata source."),
        "platform-wikimedia-commons": ("Wikimedia Commons", "Free-license image metadata and thumbnail source."),
        "platform-internet-archive": ("Internet Archive", "Public video metadata and thumbnail source."),
        "concept-ontology": ("Ontology", "Central concept connecting prompts, images, videos, creators, and platforms."),
    }
    for note_id, (title, body) in notes.items():
        (pack / "notes" / f"{note_id}.md").write_text(
            "---\n"
            "type: project\n"
            f"title: {yaml_scalar(title)}\n"
            "tags: [\"ontology\", \"source-node\"]\n"
            "---\n"
            f"{body}\n",
            encoding="utf-8",
        )


def run_pack(pack_bin: str, args: list[str], cwd: Path | None = None) -> None:
    subprocess.run(
        [pack_bin, *args],
        cwd=str(cwd) if cwd else None,
        check=True,
        stdout=sys.stderr,
    )


def materialize(records: list[Record], pack: Path, provenance: Path, download_assets: bool) -> int:
    count = 0
    for record in records:
        base_note_id = f"{record.kind}-{slug(record.source_platform)}-{slug(record.source_id)}"
        note_id = base_note_id
        suffix = 2
        while (pack / "notes" / f"{note_id}.md").exists():
            note_id = f"{base_note_id}-{suffix}"
            suffix += 1
        asset_rel = None
        if record.kind in {"image", "video"} and download_assets:
            ext = ".jpg"
            if record.kind == "image" and record.thumbnail_url.lower().split("?")[0].endswith(".png"):
                ext = ".png"
            asset_rel = f"assets/{record.kind}s/{note_id}{ext}"
            asset_path = pack / asset_rel
            if record.thumbnail_url == "fixture://transparent.png":
                asset_path.parent.mkdir(parents=True, exist_ok=True)
                asset_path.write_bytes(TRANSPARENT_PNG)
            elif not download(record.thumbnail_url, asset_path):
                asset_rel = None
        platform_id = f"platform-{slug(record.source_platform)}"
        related = ["concept-ontology", platform_id]
        write_note(pack / "notes" / f"{note_id}.md", record, asset_rel, related, str(provenance.relative_to(pack)))
        with provenance.open("a", encoding="utf-8") as f:
            f.write(json.dumps({
                "run_id": RUN_ID,
                "note_id": note_id,
                "kind": record.kind,
                "source_platform": record.source_platform,
                "source_id": record.source_id,
                "remote_url": record.remote_url,
                "thumbnail_url": record.thumbnail_url,
                "asset": asset_rel,
                "raw": record.raw,
            }, ensure_ascii=False) + "\n")
        count += 1
    return count


def collect(args: argparse.Namespace) -> tuple[list[Record], list[Record], list[Record]]:
    if args.fixture:
        return fixture_records(args.limit_each)
    prompts = civitai_prompt_records(args.prompt_limit or args.limit_each)
    images = commons_image_records(args.image_limit or args.limit_each)
    videos = archive_video_records(args.video_limit or args.limit_each)
    return prompts, images, videos


def main() -> int:
    parser = argparse.ArgumentParser(description="Create a visible OntoPack ontology/media pack from public web APIs.")
    parser.add_argument("--output", required=True, help="Pack directory to create or update")
    parser.add_argument("--pack-bin", default=os.environ.get("PACK_BIN", "pack"))
    parser.add_argument("--limit-each", type=int, default=100)
    parser.add_argument("--prompt-limit", type=int)
    parser.add_argument("--image-limit", type=int)
    parser.add_argument("--video-limit", type=int)
    parser.add_argument("--fixture", action="store_true", help="Generate deterministic offline 100/100/100 fixture records")
    parser.add_argument("--no-download-assets", action="store_true", help="Do not download thumbnails/previews into assets/")
    parser.add_argument("--build", action="store_true", help="Run pack build --no-embed after writing notes")
    args = parser.parse_args()

    pack = Path(args.output).expanduser().resolve()
    candidate_pack_bin = Path(args.pack_bin).expanduser()
    if candidate_pack_bin.exists() or candidate_pack_bin.parent != Path('.'):
        pack_bin = str(candidate_pack_bin.resolve())
    else:
        pack_bin = shutil.which(args.pack_bin) or args.pack_bin
    if not (pack / "pack.toml").exists():
        run_pack(pack_bin, ["init", str(pack)])
    for rel in ["notes", "assets", ".pack/provenance"]:
        (pack / rel).mkdir(parents=True, exist_ok=True)
    ensure_platform_notes(pack)

    provenance = pack / ".pack" / "provenance" / f"visible-ontology-pack-{RUN_ID}.jsonl"
    prompts, images, videos = collect(args)
    counts = {
        "prompt": materialize(prompts, pack, provenance, not args.no_download_assets),
        "image": materialize(images, pack, provenance, not args.no_download_assets),
        "video": materialize(videos, pack, provenance, not args.no_download_assets),
    }
    note_count = sum(1 for _ in (pack / "notes").glob("*.md"))
    asset_count = sum(1 for path in (pack / "assets").rglob("*") if path.is_file())
    provenance_count = sum(1 for _ in provenance.open("r", encoding="utf-8")) if provenance.exists() else 0
    summary = {
        "run_id": RUN_ID,
        "pack": str(pack),
        "counts": counts,
        "notes": note_count,
        "assets": asset_count,
        "provenance_records": provenance_count,
        "provenance": str(provenance.relative_to(pack)),
        "fixture": args.fixture,
        "downloaded_thumbnails": not args.no_download_assets,
        "stop_tags": sorted(STOP_TAGS),
        "low_weight_tokens": sorted(LOW_WEIGHT_TOKENS),
        "blocklist": sorted(BLOCKLIST),
    }
    summary_path = pack / ".pack" / "provenance" / f"visible-ontology-summary-{RUN_ID}.json"
    summary_path.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if args.build:
        run_pack(pack_bin, ["build", "--no-embed"], cwd=pack)
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    missing = [k for k, v in counts.items() if v < (args.limit_each if not getattr(args, f"{k}_limit", None) else getattr(args, f"{k}_limit"))]
    if missing:
        print(f"warn: collected fewer records than requested for {missing}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
