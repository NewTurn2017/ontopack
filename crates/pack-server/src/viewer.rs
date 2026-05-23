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
  <div class="vault-shell compact-ops">
    <aside class="identity-rail" aria-label="OntoPack identity and local status">
      <div class="lock-mark" aria-hidden="true"><span></span></div>
      <div class="brand-block">
        <p class="eyebrow">secure vault</p>
        <h1>OntoPack</h1>
        <p class="tagline">LOCAL OPS CONSOLE</p>
      </div>
      <div class="rail-section">
        <h2>LOCAL VAULT</h2>
        <p>검색·미디어·관계·컨텍스트를 한 화면에서 잠금 해제합니다.</p>
      </div>
      <div class="palette" aria-hidden="true">
        <span></span><span></span><span></span><span></span><span></span>
      </div>
      <div class="rail-section terminal-lines">
        <h2>SYSTEM</h2>
        <p>&gt; CORE: ONLINE</p>
        <p>&gt; NETWORK: LOCAL</p>
        <p>&gt; MODE: CONTEXT</p>
      </div>
    </aside>

    <div class="workspace">
      <header class="top-rail">
        <nav aria-label="Viewer sections">
          <span class="nav-cell active">Dashboard</span>
          <span class="nav-cell">Vault</span>
          <span class="nav-cell">Search</span>
          <span class="nav-cell">Media</span>
          <span class="nav-cell">Graph</span>
        </nav>
        <div class="user-chip" aria-label="local user status">USER: LOCAL</div>
      </header>

      <main class="dashboard-grid">
        <section class="module hero-module" aria-labelledby="dashboard-title">
          <div class="module-header">
            <div>
              <p class="eyebrow">system overview</p>
              <h2 id="dashboard-title">Dashboard</h2>
            </div>
            <span class="status-pill">STATUS: OPERATIONAL</span>
          </div>
          <div class="stat-grid" aria-label="Pack overview">
            <div class="stat-card"><span>Total types</span><strong id="stat-types">--</strong></div>
            <div class="stat-card"><span>Total tags</span><strong id="stat-tags">--</strong></div>
            <div class="stat-card"><span>Date range</span><strong id="stat-dates">--</strong></div>
            <div class="stat-card"><span>Mode</span><strong>LOCAL</strong></div>
          </div>
        </section>

        <section class="module search-module" aria-labelledby="search-title">
          <div class="module-header">
            <div>
              <p class="eyebrow">smart search</p>
              <h2 id="search-title">Vault Query Console</h2>
            </div>
            <span id="result-count" class="status-pill muted-pill">NO QUERY</span>
          </div>
          <form id="search-form" class="control-deck">
            <label class="sr-only" for="search-input">검색어</label>
            <input id="search-input" name="q" autocomplete="off" placeholder="예: 썸네일 훅, 강의 구조, 이미지 프롬프트" autofocus>
            <select id="mode-filter" aria-label="search mode"><option value="keyword">Keyword</option></select>
            <button type="submit" class="primary-action">검색</button>
            <select id="type-filter" aria-label="type filter"><option value="">모든 타입</option></select>
            <select id="tag-filter" aria-label="tag filter"><option value="">모든 태그</option></select>
            <input id="from-filter" type="date" aria-label="from date">
            <input id="to-filter" type="date" aria-label="to date">
          </form>
          <p id="filter-summary" class="panel-note">FILTERS: ALL SOURCE CARDS</p>
          <p id="mode-summary" class="panel-note">SEARCH MODE: KEYWORD · semantic disabled until server capability is available</p>
          <div id="results" class="cards source-grid"></div>
        </section>

        <section class="module ask-module" aria-labelledby="ask-title">
          <div class="module-header compact">
            <div>
              <p class="eyebrow">context terminal</p>
              <h2 id="ask-title">Ask Context</h2>
            </div>
          </div>
          <form id="ask-form" class="control-deck ask-deck">
            <label class="sr-only" for="ask-input">Ask context question</label>
            <input id="ask-input" name="q" autocomplete="off" placeholder="예: 이 팩에서 썸네일 훅 자료 요약해줘">
            <button type="submit" class="secondary-action">Ask 컨텍스트</button>
          </form>
          <div id="ask-context" class="cards terminal-output muted">질문하면 citation-ready context blocks를 보여줍니다.</div>
        </section>

        <section class="module note-module" aria-labelledby="note-title">
          <div class="module-header compact">
            <div>
              <p class="eyebrow">selected record</p>
              <h2 id="note-title">노트</h2>
            </div>
          </div>
          <div id="note-detail" class="note-detail muted">검색 결과를 클릭하면 노트가 열립니다.</div>
        </section>

        <section class="module related-module" aria-labelledby="related-title">
          <div class="module-header compact">
            <div>
              <p class="eyebrow">linked records</p>
              <h2 id="related-title">관련 노트</h2>
            </div>
          </div>
          <div id="related" class="cards muted">노트 선택 대기 중</div>
        </section>

        <section class="module gallery-panel" aria-labelledby="gallery-title">
          <div class="module-header compact">
            <div>
              <p class="eyebrow">asset bay</p>
              <h2 id="gallery-title">갤러리</h2>
            </div>
          </div>
          <div id="gallery" class="cards"></div>
        </section>

        <section class="module timeline-module" aria-labelledby="timeline-title">
          <div class="module-header compact">
            <div>
              <p class="eyebrow">archive log</p>
              <h2 id="timeline-title">타임라인</h2>
            </div>
          </div>
          <div id="timeline" class="cards"></div>
        </section>

        <section class="module graph-module" aria-labelledby="graph-title">
          <div class="module-header compact">
            <div>
              <p class="eyebrow">relation map</p>
              <h2 id="graph-title">그래프</h2>
            </div>
          </div>
          <div id="graph" class="graph"></div>
        </section>
      </main>
    </div>
  </div>
  <script src="/app.js"></script>
</body>
</html>
"#
}

pub fn app_js() -> &'static str {
    r#"const $ = (id) => document.getElementById(id);

const requestControllers = new Map();

async function fetchJson(url, options = {}) {
  const response = await fetch(url, options);
  const payload = await response.json();
  if (!response.ok) throw new Error(payload.error || response.statusText);
  return payload;
}

function nextSignal(key) {
  const existing = requestControllers.get(key);
  if (existing) existing.abort();
  const controller = new AbortController();
  requestControllers.set(key, controller);
  return controller.signal;
}

function clearSignal(key, signal) {
  const controller = requestControllers.get(key);
  if (controller && controller.signal === signal) {
    requestControllers.delete(key);
    return true;
  }
  return false;
}

function isAbort(error) {
  return error && error.name === 'AbortError';
}

function debounce(fn, delay = 180) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args).catch((error) => {
      if (!isAbort(error)) console.error(error);
    }), delay);
  };
}

function escapeHtml(value) {
  return String(value ?? '').replace(/[&<>"']/g, (c) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
}

function mediaMarkup(item, size = 'thumb', interactive = false) {
  if (!item || !item.asset_url) return '';
  const title = escapeHtml(item.title || item.asset || 'media asset');
  const src = escapeHtml(item.asset_url);
  const kind = item.media_kind || '';
  if (kind === 'image') {
    return `<figure class="media-preview media-${size}"><img src="${src}" alt="${title}" loading="lazy" decoding="async"></figure>`;
  }
  if (kind === 'video') {
    return interactive
      ? `<figure class="media-preview media-${size}"><video src="${src}" controls preload="metadata" title="${title}"></video></figure>`
      : `<figure class="media-preview media-${size} media-token"><span>VIDEO</span></figure>`;
  }
  if (kind === 'audio') {
    return interactive
      ? `<figure class="media-preview media-${size}"><audio src="${src}" controls preload="metadata" title="${title}"></audio></figure>`
      : `<figure class="media-preview media-${size} media-token"><span>AUDIO</span></figure>`;
  }
  return `<figure class="media-preview media-${size} media-token"><span>${escapeHtml((item.mime || 'FILE').split('/').pop().toUpperCase())}</span></figure>`;
}

function card(hit) {
  const id = hit.note_id || hit.id;
  const type = hit.note_type || 'record';
  const meta = hit.chunk_id || hit.id || '';
  return `<button class="card source-card type-${escapeHtml(type)}" data-note-id="${escapeHtml(id)}">
    ${mediaMarkup(hit, 'thumb', false)}
    <span class="card-kicker">${escapeHtml(type)}</span>
    <strong>${escapeHtml(hit.title)}</strong>
    <span class="meta-line">${escapeHtml(meta)}</span>
    <p>${escapeHtml(hit.snippet || hit.caption || hit.created || '')}</p>
  </button>`;
}

function galleryCard(item) {
  return `<article class="card gallery-card" data-note-id="${escapeHtml(item.id)}" role="button" tabindex="0">
    ${mediaMarkup(item, 'gallery', item.media_kind === 'video' || item.media_kind === 'audio')}
    ${keyframeStrip(item, false)}
    <span class="card-kicker">${escapeHtml(item.note_type)}</span>
    <strong>${escapeHtml(item.title)}</strong>
    <span class="meta-line">${escapeHtml(item.asset || '')}</span>
    <p>${escapeHtml(item.caption || '')}</p>
  </article>`;
}

function keyframeStrip(item, interactive = true) {
  const frames = (item && item.keyframes || []).filter((frame) => frame && (frame.asset_url || frame.text));
  if (!frames.length) return '';
  const cards = frames.slice(0, 6).map((frame) => {
    const time = escapeHtml(frame.time || '');
    const label = escapeHtml(frame.text || 'keyframe');
    const src = frame.asset_url ? escapeHtml(frame.asset_url) : '';
    const media = src
      ? `<img src="${src}" alt="${label}" loading="lazy" decoding="async">`
      : `<span class="keyframe-token">FRAME</span>`;
    return `<figure class="keyframe-card" title="${label}">
      ${media}
      <figcaption>${time}</figcaption>
    </figure>`;
  }).join('');
  const heading = interactive ? '<p class="meta keyframe-heading">KEYFRAMES</p>' : '';
  return `<div class="keyframe-strip" aria-label="video keyframes">${heading}${cards}</div>`;
}

function filterParams() {
  const params = new URLSearchParams();
  const type = $('type-filter').value;
  const tag = $('tag-filter').value;
  const from = $('from-filter').value;
  const to = $('to-filter').value;
  if (type) params.set('type', type);
  if (tag) params.set('tag', tag);
  if (from) params.set('from', from);
  if (to) params.set('to', to);
  return params;
}

function updateFilterSummary() {
  const parts = [];
  const type = $('type-filter').value;
  const tag = $('tag-filter').value;
  const from = $('from-filter').value;
  const to = $('to-filter').value;
  if (type) parts.push(`TYPE=${type}`);
  if (tag) parts.push(`TAG=${tag}`);
  if (from) parts.push(`FROM=${from}`);
  if (to) parts.push(`TO=${to}`);
  $('filter-summary').textContent = parts.length ? `FILTERS: ${parts.join(' · ')}` : 'FILTERS: ALL SOURCE CARDS';
}

function renderCapabilities(caps) {
  const modes = caps.search_modes || [];
  $('mode-filter').innerHTML = modes.map((mode) => {
    const disabled = mode.available ? '' : ' disabled';
    const label = mode.available ? mode.mode : `${mode.mode} (locked)`;
    const reason = mode.reason ? ` title="${escapeHtml(mode.reason)}"` : '';
    return `<option value="${escapeHtml(mode.mode)}"${disabled}${reason}>${escapeHtml(label)}</option>`;
  }).join('');
  $('mode-filter').value = caps.default_search_mode || 'keyword';
  const locked = modes.filter((mode) => !mode.available).map((mode) => mode.mode).join('/');
  $('mode-summary').textContent = caps.semantic_search
    ? `SEARCH MODE: ${$('mode-filter').value.toUpperCase()} · semantic enabled`
    : `SEARCH MODE: KEYWORD · ${locked || 'semantic'} locked until server capability is available`;
}

async function loadCapabilities() {
  const caps = await fetchJson('/api/capabilities');
  renderCapabilities(caps);
}

function setLoading(id, loading) {
  $(id).classList.toggle('is-loading', loading);
}

function setPanelLoading(loading) {
  for (const id of ['gallery', 'timeline', 'graph']) setLoading(id, loading);
}

function setResultCount(count, querying = false, elapsedMs = null) {
  const timing = elapsedMs === null || elapsedMs === undefined ? '' : ` · ${elapsedMs}ms`;
  $('result-count').textContent = querying ? 'QUERYING...' : `${count} SOURCE CARD${count === 1 ? '' : 'S'}${timing}`;
  $('result-count').classList.toggle('muted-pill', !querying && count === 0);
}

function renderFacets(facets) {
  const currentType = $('type-filter').value;
  const currentTag = $('tag-filter').value;
  $('type-filter').innerHTML = '<option value="">모든 타입</option>' + facets.types.map((type) => `<option value="${escapeHtml(type)}">${escapeHtml(type)}</option>`).join('');
  $('tag-filter').innerHTML = '<option value="">모든 태그</option>' + facets.tags.map((tag) => `<option value="${escapeHtml(tag)}">#${escapeHtml(tag)}</option>`).join('');
  if (currentType && facets.types.includes(currentType)) $('type-filter').value = currentType;
  if (currentTag && facets.tags.includes(currentTag)) $('tag-filter').value = currentTag;
  $('stat-types').textContent = facets.types.length;
  $('stat-tags').textContent = facets.tags.length;
  $('stat-dates').textContent = facets.created_min && facets.created_max ? `${facets.created_min.slice(5)}–${facets.created_max.slice(5)}` : 'NO DATE';
}

async function loadDashboard() {
  const signal = nextSignal('dashboard');
  setPanelLoading(true);
  const params = filterParams();
  params.set('gallery_k', '12');
  params.set('timeline_k', '10');
  params.set('graph_limit', '80');
  try {
    const data = await fetchJson(`/api/dashboard?${params.toString()}`, { signal });
    renderFacets(data.facets);
    renderTimeline(data.timeline);
    renderGallery(data.gallery);
    renderGraph(data.graph);
  } catch (error) {
    if (!isAbort(error)) throw error;
  } finally {
    if (clearSignal('dashboard', signal)) setPanelLoading(false);
  }
}

async function search(q) {
  const signal = nextSignal('search');
  updateFilterSummary();
  setResultCount(0, true);
  setLoading('results', true);
  const params = filterParams();
  params.set('mode', $('mode-filter').value || 'keyword');
  params.set('q', q);
  params.set('k', '12');
  try {
    const data = await fetchJson(`/api/search?${params.toString()}`, { signal });
    $('results').innerHTML = data.hits.length ? data.hits.map(card).join('') : '<p class="muted empty-state">NO MATCHING SOURCE CARDS</p>';
    $('mode-summary').textContent = `SEARCH MODE: ${data.mode.toUpperCase()} · ${data.source} · ${data.elapsed_ms}ms`;
    setResultCount(data.hits.length, false, data.elapsed_ms);
  } catch (error) {
    if (!isAbort(error)) throw error;
  } finally {
    if (clearSignal('search', signal)) setLoading('results', false);
  }
}

async function ask(q) {
  const signal = nextSignal('ask');
  $('ask-context').classList.remove('muted');
  $('ask-context').innerHTML = '<p class="terminal-line">QUERYING CONTEXT BLOCKS...</p>';
  try {
    const data = await fetchJson(`/api/ask?q=${encodeURIComponent(q)}&k=5`, { signal });
    $('ask-context').innerHTML = `<p class="meta terminal-line">${escapeHtml(data.answer_mode)} · ${escapeHtml(data.instruction)} · ${data.elapsed_ms}ms</p>` +
      (data.context_blocks.length ? data.context_blocks.map(card).join('') : '<p class="muted empty-state">컨텍스트 없음</p>');
  } catch (error) {
    if (!isAbort(error)) throw error;
  } finally {
    clearSignal('ask', signal);
  }
}

async function openNote(id) {
  document.querySelectorAll('[data-note-id]').forEach((el) => el.classList.toggle('selected', el.dataset.noteId === id));
  const note = await fetchJson(`/api/notes/${encodeURIComponent(id)}`);
  $('note-detail').classList.remove('muted');
  $('note-detail').innerHTML = `${mediaMarkup(note, 'large', true)}
    ${keyframeStrip(note, true)}
    <h3>${escapeHtml(note.title)}</h3>
    <p class="meta">${escapeHtml(note.note_type)} · ${escapeHtml(note.created || 'no date')} · ${escapeHtml(note.tags.join(', '))}</p>
    <pre>${escapeHtml(note.body)}</pre>`;
  const related = await fetchJson(`/api/related/${encodeURIComponent(id)}?depth=1`);
  $('related').classList.remove('muted');
  $('related').innerHTML = related.related.length ? related.related.map((n) => card({ ...n, chunk_id: `depth ${n.depth}` })).join('') : '<p class="muted empty-state">관련 노트 없음</p>';
}

function renderTimeline(data) {
  $('timeline').innerHTML = data.notes.length ? data.notes.map((n) => card({ ...n, chunk_id: n.id, snippet: n.created || '' })).join('') : '<p class="muted empty-state">created 메타데이터가 있는 노트 없음</p>';
}

function renderGallery(data) {
  $('gallery').innerHTML = data.items.length ? data.items.map(galleryCard).join('') : '<p class="muted empty-state">asset 사이드카 노트 없음</p>';
}

function renderGraph(graph) {
  $('graph').innerHTML = `<p class="graph-count">${graph.nodes.length} nodes · ${graph.edges.length} links</p>` + graph.edges.slice(0, 80).map((e) => `<span>${escapeHtml(e.from)} → ${escapeHtml(e.to)}</span>`).join('');
}

async function refreshPanels() {
  updateFilterSummary();
  await loadDashboard();
}

async function refreshForFilters() {
  const q = $('search-input').value.trim();
  await Promise.all([q ? search(q) : Promise.resolve(), refreshPanels()]);
}

const debouncedSearch = debounce(async () => {
  const q = $('search-input').value.trim();
  if (q) {
    await search(q);
  } else {
    $('results').innerHTML = '';
    setResultCount(0);
  }
});

$('search-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  const q = $('search-input').value.trim();
  if (q) await search(q);
  await refreshPanels();
});

$('ask-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  const q = $('ask-input').value.trim();
  if (q) await ask(q);
});

for (const id of ['type-filter', 'tag-filter', 'from-filter', 'to-filter', 'mode-filter']) {
  $(id).addEventListener('change', refreshForFilters);
}
$('search-input').addEventListener('input', debouncedSearch);

document.body.addEventListener('click', async (event) => {
  if (event.target.closest('video, audio, a')) return;
  const target = event.target.closest('[data-note-id]');
  if (target) await openNote(target.dataset.noteId);
});

document.body.addEventListener('keydown', async (event) => {
  if (event.key !== 'Enter' && event.key !== ' ') return;
  const target = event.target.closest('[data-note-id]');
  if (!target) return;
  event.preventDefault();
  await openNote(target.dataset.noteId);
});

Promise.all([loadCapabilities(), loadDashboard()]).catch(console.error);
"#
}

pub fn style_css() -> &'static str {
    r#":root {
  color-scheme: dark;
  --bg: #05090a;
  --bg-2: #081113;
  --metal: #101719;
  --metal-2: #151f22;
  --metal-3: #202b2e;
  --line: rgba(97, 255, 174, .22);
  --line-hard: rgba(97, 255, 174, .45);
  --green: #00f99a;
  --green-2: #32d583;
  --cyan: #4cc9f0;
  --text: #d8e5e0;
  --muted: #81938e;
  --danger: #ff5c5c;
  --shadow: 0 22px 80px rgba(0,0,0,.45), inset 0 1px 0 rgba(255,255,255,.05);
  font-family: -apple-system, BlinkMacSystemFont, "Apple SD Gothic Neo", "Noto Sans KR", system-ui, sans-serif;
}
* { box-sizing: border-box; }
html { min-height: 100%; background: var(--bg); }
body {
  min-height: 100vh;
  margin: 0;
  color: var(--text);
  font-size: 15px;
  line-height: 1.55;
  background:
    radial-gradient(circle at 50% -20%, rgba(0,249,154,.15), transparent 35%),
    linear-gradient(120deg, rgba(76,201,240,.06), transparent 35%),
    linear-gradient(rgba(255,255,255,.025) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255,255,255,.02) 1px, transparent 1px),
    var(--bg);
  background-size: auto, auto, 42px 42px, 42px 42px, auto;
}
body::before {
  content: "";
  position: fixed;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(transparent 50%, rgba(0,0,0,.18) 51%);
  background-size: 100% 4px;
  opacity: .28;
  mix-blend-mode: multiply;
}
a, button, input, select { font: inherit; }
button, input, select { color: inherit; }
button { cursor: pointer; }
button:focus-visible, input:focus-visible, select:focus-visible {
  outline: 2px solid var(--green);
  outline-offset: 3px;
  box-shadow: 0 0 0 6px rgba(0,249,154,.12);
}
.vault-shell {
  display: grid;
  grid-template-columns: minmax(176px, 210px) minmax(0, 1fr);
  gap: 12px;
  min-height: 100vh;
  padding: 12px;
}
.compact-ops { --panel-max: min(24vh, 220px); }
.identity-rail, .top-rail, .module {
  position: relative;
  border: 1px solid rgba(164, 255, 218, .16);
  background: linear-gradient(145deg, rgba(18,27,29,.96), rgba(7,13,15,.96));
  box-shadow: var(--shadow);
}
.identity-rail::before, .top-rail::before, .module::before {
  content: "";
  position: absolute;
  inset: 7px;
  pointer-events: none;
  border: 1px solid rgba(0,249,154,.08);
}
.identity-rail {
  overflow: hidden;
  padding: 18px 14px;
  border-radius: 18px;
  clip-path: polygon(0 14px, 14px 0, 100% 0, 100% calc(100% - 14px), calc(100% - 14px) 100%, 0 100%);
}
.identity-rail::after {
  content: "";
  position: absolute;
  inset: auto -20% 0 -20%;
  height: 35%;
  background: repeating-linear-gradient(60deg, rgba(0,249,154,.08), rgba(0,249,154,.08) 1px, transparent 1px, transparent 14px);
  opacity: .4;
}
.lock-mark {
  position: relative;
  width: 58px;
  height: 58px;
  margin-bottom: 12px;
  border-radius: 50%;
  border: 2px solid rgba(216,229,224,.42);
  background: radial-gradient(circle, rgba(0,249,154,.18) 0 18%, rgba(8,15,17,.95) 19% 55%, rgba(255,255,255,.08) 56% 58%, transparent 59%);
  box-shadow: inset 0 0 24px rgba(0,0,0,.9), 0 0 28px rgba(0,249,154,.12);
}
.lock-mark span, .lock-mark::before, .lock-mark::after { content: ""; position: absolute; inset: 10px; border: 1px solid rgba(216,229,224,.28); border-radius: 50%; }
.lock-mark::before { inset: 20px; background: #0d1416; }
.lock-mark::after { inset: 28px 12px; border-radius: 999px; background: var(--green); box-shadow: 0 0 12px var(--green); }
.brand-block h1 {
  margin: 0;
  font-size: clamp(24px, 2.4vw, 34px);
  letter-spacing: .03em;
  line-height: 1;
  text-shadow: 0 0 18px rgba(0,249,154,.2);
}
.eyebrow, .tagline, .nav-cell, .user-chip, .status-pill, .panel-note, .card-kicker, .meta, .meta-line, .terminal-lines, .graph-count {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
}
.eyebrow { margin: 0 0 5px; color: var(--green); text-transform: uppercase; letter-spacing: .12em; font-size: 11px; }
.tagline { margin: 8px 0 0; color: var(--green-2); letter-spacing: .16em; font-size: 11px; }
.rail-section { position: relative; z-index: 1; margin-top: 18px; padding-top: 14px; border-top: 1px solid rgba(164,255,218,.12); }
.rail-section h2 { margin: 0 0 8px; font-size: 12px; letter-spacing: .08em; text-transform: uppercase; }
.rail-section p { margin: 4px 0; color: var(--muted); font-size: 13px; line-height: 1.45; }
.palette { display: grid; grid-template-columns: repeat(5, 1fr); gap: 6px; margin: 16px 0; }
.palette span { height: 22px; border: 1px solid rgba(255,255,255,.18); background: #0b0f11; }
.palette span:nth-child(2) { background: #151b1f; }
.palette span:nth-child(3) { background: #1f2a2e; }
.palette span:nth-child(4) { background: var(--green-2); box-shadow: 0 0 18px rgba(0,249,154,.32); }
.palette span:nth-child(5) { background: #d4ddd9; }
.workspace { min-width: 0; }
.top-rail {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  min-height: 54px;
  padding: 8px 12px;
  margin-bottom: 12px;
  border-radius: 18px;
  clip-path: polygon(0 12px, 12px 0, 100% 0, 100% calc(100% - 12px), calc(100% - 12px) 100%, 0 100%);
}
.top-rail nav { display: flex; flex-wrap: wrap; gap: 6px; }
.nav-cell, .user-chip, .status-pill {
  border: 1px solid rgba(164,255,218,.16);
  background: linear-gradient(180deg, rgba(255,255,255,.045), rgba(0,0,0,.16));
  color: var(--muted);
  padding: 7px 11px;
  text-transform: uppercase;
  letter-spacing: .08em;
  font-size: 12px;
}
.nav-cell.active, .status-pill { color: var(--green); border-color: var(--line-hard); box-shadow: inset 0 -2px 0 rgba(0,249,154,.55); }
.nav-cell.disabled { opacity: .55; }
.user-chip { white-space: nowrap; }
.dashboard-grid {
  display: grid;
  grid-template-columns: minmax(360px, 1.12fr) minmax(250px, .78fr) minmax(250px, .72fr);
  grid-auto-flow: dense;
  gap: 12px;
}
.module {
  min-width: 0;
  padding: 12px;
  border-radius: 14px;
  clip-path: polygon(0 14px, 14px 0, 100% 0, 100% calc(100% - 14px), calc(100% - 14px) 100%, 0 100%);
}
.module-header { position: relative; z-index: 1; display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; margin-bottom: 10px; }
.module-header h2 { margin: 0; font-size: 19px; text-transform: uppercase; letter-spacing: .04em; }
.module-header.compact h2 { font-size: 15px; }
.hero-module { grid-column: 1 / -1; }
.search-module { grid-column: 1 / 2; grid-row: 2 / 4; }
.note-module { grid-column: 2 / 3; grid-row: 2 / 3; }
.gallery-panel { grid-column: 3 / 4; grid-row: 2 / 3; }
.timeline-module { grid-column: 2 / 3; grid-row: 3 / 4; }
.graph-module { grid-column: 3 / 4; grid-row: 3 / 4; }
.ask-module { grid-column: 1 / 3; grid-row: 4 / 5; }
.related-module { grid-column: 3 / 4; grid-row: 4 / 5; }
.stat-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
.stat-card {
  min-height: 64px;
  padding: 10px;
  border: 1px solid rgba(164,255,218,.12);
  background: rgba(0,0,0,.24);
  box-shadow: inset 0 0 24px rgba(0,0,0,.38);
}
.stat-card span { display: block; color: var(--muted); font-size: 12px; text-transform: uppercase; }
.stat-card strong { display: block; margin-top: 4px; font-size: clamp(18px, 1.5vw, 22px); font-weight: 600; }
.control-deck { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
.control-deck input, .control-deck select { flex: 1 1 116px; }
#search-input, #ask-input { flex: 999 1 240px; }
.ask-deck { flex-wrap: nowrap; }
input, select {
  min-height: 38px;
  border: 1px solid rgba(0,249,154,.25);
  border-radius: 8px;
  padding: 8px 10px;
  background: linear-gradient(180deg, rgba(0,0,0,.42), rgba(9,18,20,.82));
  color: var(--text);
}
input::placeholder { color: rgba(216,229,224,.45); }
.primary-action, .secondary-action {
  min-height: 38px;
  border: 1px solid var(--line-hard);
  border-radius: 8px;
  min-width: max-content;
  padding: 0 13px;
  color: var(--green);
  white-space: nowrap;
  background: linear-gradient(180deg, rgba(0,249,154,.16), rgba(0,0,0,.34));
  text-transform: uppercase;
  letter-spacing: .08em;
  font-weight: 800;
}
.secondary-action { color: var(--cyan); border-color: rgba(76,201,240,.35); }
.panel-note { color: var(--muted); font-size: 11px; letter-spacing: .08em; margin: 7px 0; }
.cards { display: grid; gap: 8px; }
.source-grid { margin-top: 8px; max-height: var(--panel-max); overflow: auto; padding-right: 3px; }
#gallery, #timeline, #graph, #related, #note-detail { max-height: var(--panel-max); overflow: auto; padding-right: 3px; }
.note-detail { font-size: 13px; }
.source-grid::-webkit-scrollbar, #gallery::-webkit-scrollbar, #timeline::-webkit-scrollbar, #graph::-webkit-scrollbar, #related::-webkit-scrollbar, #note-detail::-webkit-scrollbar, #ask-context::-webkit-scrollbar { width: 7px; }
.source-grid::-webkit-scrollbar-thumb, #gallery::-webkit-scrollbar-thumb, #timeline::-webkit-scrollbar-thumb, #graph::-webkit-scrollbar-thumb, #related::-webkit-scrollbar-thumb, #note-detail::-webkit-scrollbar-thumb, #ask-context::-webkit-scrollbar-thumb { background: rgba(0,249,154,.26); border-radius: 999px; }
.is-loading { opacity: .62; filter: saturate(.75); }
.card {
  position: relative;
  display: block;
  width: 100%;
  text-align: left;
  border: 1px solid rgba(164,255,218,.14);
  border-radius: 10px;
  background: linear-gradient(145deg, rgba(16,24,26,.88), rgba(7,12,13,.92));
  color: inherit;
  padding: 9px 10px 9px 12px;
  box-shadow: inset 3px 0 0 rgba(0,249,154,.25);
}
.card:hover, .card.selected { border-color: var(--line-hard); transform: translateY(-1px); box-shadow: inset 3px 0 0 var(--green), 0 0 22px rgba(0,249,154,.12); }
.card strong { display: block; margin: 1px 0; font-size: 14px; }
.card p { margin: 5px 0 0; color: #b9c8c3; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.card-kicker { color: var(--green); font-size: 11px; text-transform: uppercase; letter-spacing: .12em; }
.meta-line, .meta, .muted { color: var(--muted); }
.empty-state { margin: 0; padding: 14px; border: 1px dashed rgba(164,255,218,.16); }
.media-preview {
  position: relative;
  overflow: hidden;
  margin: 0 0 12px;
  border: 1px solid rgba(0,249,154,.2);
  border-radius: 10px;
  background:
    linear-gradient(135deg, rgba(0,249,154,.12), transparent 38%),
    repeating-linear-gradient(90deg, rgba(255,255,255,.035), rgba(255,255,255,.035) 1px, transparent 1px, transparent 12px),
    #071012;
  box-shadow: inset 0 0 22px rgba(0,0,0,.5);
}
.media-preview::after {
  content: "MEDIA";
  position: absolute;
  right: 8px;
  bottom: 6px;
  color: rgba(0,249,154,.72);
  font: 10px ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  letter-spacing: .12em;
  pointer-events: none;
}
.media-preview img, .media-preview video {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  background: #020607;
}
.media-preview audio { display: block; width: 100%; margin: 26px 0 12px; }
.media-thumb { float: right; width: 88px; height: 58px; margin: 0 0 8px 10px; }
.media-gallery { aspect-ratio: 16 / 9; min-height: 92px; }
.media-large { aspect-ratio: 16 / 9; max-height: 260px; }
.media-token { display: grid; place-items: center; min-height: 78px; }
.media-token span {
  color: var(--green);
  border: 1px solid rgba(0,249,154,.28);
  padding: 8px 12px;
  background: rgba(0,0,0,.28);
  font-weight: 800;
  letter-spacing: .12em;
}
.gallery-card { cursor: pointer; }
.gallery-card video { cursor: auto; }
.keyframe-strip {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(72px, 1fr));
  gap: 7px;
  margin: 8px 0 10px;
}
.keyframe-heading { grid-column: 1 / -1; margin: 0; color: var(--green); letter-spacing: .14em; }
.keyframe-card {
  position: relative;
  overflow: hidden;
  margin: 0;
  min-height: 54px;
  border: 1px solid rgba(0,249,154,.16);
  border-radius: 10px;
  background: rgba(0,0,0,.34);
}
.keyframe-card img {
  display: block;
  width: 100%;
  height: 62px;
  object-fit: cover;
  filter: saturate(.92) contrast(1.08);
}
.keyframe-card figcaption {
  position: absolute;
  left: 5px;
  bottom: 4px;
  padding: 2px 5px;
  border-radius: 999px;
  color: #04130f;
  background: rgba(0,249,154,.82);
  font: 10px ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
  font-weight: 800;
}
.keyframe-token {
  display: grid;
  place-items: center;
  min-height: 62px;
  color: var(--green);
  font-size: 11px;
  letter-spacing: .13em;
}
.terminal-output { min-height: 74px; max-height: 220px; overflow: auto; }
.terminal-line { color: var(--green); }
.note-detail h3 { margin: 0 0 6px; color: #fff; }
pre {
  white-space: pre-wrap;
  margin: 8px 0 0;
  padding: 10px;
  border: 1px solid rgba(164,255,218,.12);
  border-radius: 12px;
  background: rgba(0,0,0,.28);
  color: #c8d6d1;
  overflow: auto;
}
.graph span { display: inline-block; margin: 3px; padding: 4px 7px; border-radius: 999px; background: rgba(0,249,154,.08); color: var(--green); border: 1px solid rgba(0,249,154,.16); font-size: 12px; }
.muted-pill { color: var(--muted); border-color: rgba(164,255,218,.14); box-shadow: none; }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0; }
@media (max-width: 1180px) {
  .vault-shell { grid-template-columns: 1fr; }
  .identity-rail { display: grid; grid-template-columns: auto 1fr; gap: 10px 16px; }
  .rail-section, .palette { grid-column: 1 / -1; }
}
@media (max-width: 860px) {
  .vault-shell { padding: 10px; }
  .dashboard-grid, .hero-module, .search-module, .ask-module, .note-module, .related-module, .gallery-panel, .timeline-module, .graph-module { display: block; grid-column: auto; grid-row: auto; }
  .module { margin-bottom: 14px; }
  .control-deck, .ask-deck { display: grid; grid-template-columns: 1fr; }
  .stat-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero-module { grid-column: auto; }
  .top-rail { align-items: stretch; flex-direction: column; }
}
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { transition: none !important; animation: none !important; }
}
"#
}
