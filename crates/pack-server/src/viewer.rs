pub fn index_html() -> &'static str {
    r#"<!doctype html>
<html lang="ko">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ontopack viewer</title>
  <link rel="stylesheet" href="/style.css">
</head>
<body>
  <header>
    <p class="eyebrow">local ontology knowledge pack</p>
    <h1>ontopack</h1>
    <p>로컬 팩의 노트, 검색 카드, 관련 링크, 타임라인을 빠르게 탐색합니다.</p>
  </header>
  <main>
    <section class="panel search-panel">
      <form id="search-form">
        <input id="search-input" name="q" autocomplete="off" placeholder="예: 썸네일 훅, 강의 구조, 이미지 프롬프트" autofocus>
        <button type="submit">검색</button>
      </form>
      <div id="results" class="cards"></div>
    </section>
    <section class="grid">
      <article class="panel">
        <h2>노트</h2>
        <div id="note-detail" class="note-detail muted">검색 결과를 클릭하면 노트가 열립니다.</div>
      </article>
      <article class="panel">
        <h2>관련 노트</h2>
        <div id="related" class="cards muted">노트 선택 대기 중</div>
      </article>
    </section>
    <section class="grid">
      <article class="panel">
        <h2>타임라인</h2>
        <div id="timeline" class="cards"></div>
      </article>
      <article class="panel">
        <h2>그래프</h2>
        <div id="graph" class="graph"></div>
      </article>
    </section>
  </main>
  <script src="/app.js"></script>
</body>
</html>
"#
}

pub fn app_js() -> &'static str {
    r#"const $ = (id) => document.getElementById(id);

async function fetchJson(url) {
  const response = await fetch(url);
  const payload = await response.json();
  if (!response.ok) throw new Error(payload.error || response.statusText);
  return payload;
}

function escapeHtml(value) {
  return String(value ?? '').replace(/[&<>"']/g, (c) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
}

function card(hit) {
  return `<button class="card" data-note-id="${escapeHtml(hit.note_id)}">
    <strong>${escapeHtml(hit.title)}</strong>
    <span>${escapeHtml(hit.note_type)} · ${escapeHtml(hit.chunk_id || hit.id || '')}</span>
    <p>${escapeHtml(hit.snippet || hit.created || '')}</p>
  </button>`;
}

async function search(q) {
  const data = await fetchJson(`/api/search?q=${encodeURIComponent(q)}&k=12`);
  $('results').innerHTML = data.hits.length ? data.hits.map(card).join('') : '<p class="muted">검색 결과 없음</p>';
}

async function openNote(id) {
  const note = await fetchJson(`/api/notes/${encodeURIComponent(id)}`);
  $('note-detail').classList.remove('muted');
  $('note-detail').innerHTML = `<h3>${escapeHtml(note.title)}</h3>
    <p class="meta">${escapeHtml(note.note_type)} · ${escapeHtml(note.created || 'no date')} · ${escapeHtml(note.tags.join(', '))}</p>
    <pre>${escapeHtml(note.body)}</pre>`;
  const related = await fetchJson(`/api/related/${encodeURIComponent(id)}?depth=1`);
  $('related').classList.remove('muted');
  $('related').innerHTML = related.related.length ? related.related.map((n) => card({ ...n, chunk_id: `depth ${n.depth}` })).join('') : '<p class="muted">관련 노트 없음</p>';
}

async function loadTimeline() {
  const data = await fetchJson('/api/timeline?k=10');
  $('timeline').innerHTML = data.notes.length ? data.notes.map((n) => card({ ...n, chunk_id: n.id, snippet: n.created || '' })).join('') : '<p class="muted">created 메타데이터가 있는 노트 없음</p>';
}

async function loadGraph() {
  const graph = await fetchJson('/api/graph?limit=80');
  $('graph').innerHTML = `<p>${graph.nodes.length} nodes · ${graph.edges.length} links</p>` + graph.edges.slice(0, 80).map((e) => `<span>${escapeHtml(e.from)} → ${escapeHtml(e.to)}</span>`).join('');
}

$('search-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  const q = $('search-input').value.trim();
  if (q) await search(q);
});

document.body.addEventListener('click', async (event) => {
  const target = event.target.closest('[data-note-id]');
  if (target) await openNote(target.dataset.noteId);
});

loadTimeline().catch(console.error);
loadGraph().catch(console.error);
"#
}

pub fn style_css() -> &'static str {
    r#":root { color-scheme: light; font-family: -apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", "Noto Sans KR", sans-serif; }
body { margin: 0; background: #f6f4ef; color: #222; font-size: 15px; line-height: 1.55; }
header { padding: 40px max(24px, 8vw) 24px; background: #171717; color: #fff; }
.eyebrow { margin: 0; color: #b7f7d0; text-transform: uppercase; letter-spacing: .12em; font-size: 12px; }
h1 { margin: 6px 0 8px; font-size: clamp(32px, 6vw, 64px); }
main { padding: 24px max(18px, 6vw) 48px; }
.panel { background: #fff; border: 1px solid #e6e1d6; border-radius: 18px; padding: 18px; margin-bottom: 18px; }
.grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 18px; }
form { display: flex; gap: 10px; }
input { flex: 1; border: 1px solid #d8d0c1; border-radius: 14px; padding: 14px 16px; font-size: 16px; }
button { cursor: pointer; }
form button { border: 0; border-radius: 14px; padding: 0 18px; background: #1d6b45; color: #fff; font-weight: 700; }
.cards { display: grid; gap: 10px; }
.card { display: block; width: 100%; text-align: left; border: 1px solid #e8e3d8; border-radius: 14px; background: #fffdf8; padding: 12px; color: inherit; }
.card strong { display: block; font-size: 16px; }
.card span, .meta, .muted { color: #706a60; }
.card p { margin: 8px 0 0; }
pre { white-space: pre-wrap; background: #f7f7f7; padding: 12px; border-radius: 12px; overflow: auto; }
.graph span { display: inline-block; margin: 4px; padding: 5px 8px; border-radius: 999px; background: #edf7ef; color: #29543a; }
"#
}
