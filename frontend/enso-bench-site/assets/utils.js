export async function fetchJSON(url) {
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`Failed to fetch ${url}: ${resp.status}`);
  return resp.json();
}

export function readFileAsText(file) {
  return new Promise((resolve, reject) => {
    const fr = new FileReader();
    fr.onerror = () => reject(fr.error);
    fr.onload = () => resolve(String(fr.result || ""));
    fr.readAsText(file);
  });
}

export function parseJSONL(text) {
  return text
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line, i) => {
      try {
        return JSON.parse(line);
      } catch (err) {
        console.warn('Bad JSONL line', i + 1, err);
        return null;
      }
    })
    .filter(Boolean);
}

export function fmtScore(n) {
  const x = Number(n ?? 0);
  return (Math.round(x * 100) / 100).toFixed(2);
}

export function actionLabel(sig = {}) {
  const chain = sig.chain_id != null ? `chain ${sig.chain_id}` : 'chain ?';
  const protocol = sig.protocol ? ` · ${sig.protocol}` : '';
  const tokens = Array.isArray(sig.tokens) && sig.tokens.length
    ? ` (${sig.tokens.join(' → ')})`
    : '';
  return `${chain} · ${sig.action || 'action'}${protocol}${tokens}`;
}

const DOMAIN_TAGS = {
  dex: 'tag-dex',
  lending: 'tag-lending',
  yield: 'tag-yield',
  bridge: 'tag-bridge',
  derivatives: 'tag-derivatives'
};

export function domainPill(name) {
  if (!name) return '';
  const key = String(name).toLowerCase();
  const cls = DOMAIN_TAGS[key] || 'tag-default';
  return `<span class="tag ${cls}">${key}</span>`;
}

const NEEDLE_TAGS = {
  PASS: 'tag-pass',
  PASS_WITH_WARNINGS: 'tag-warning',
  PARTIAL: 'tag-partial',
  FAIL: 'tag-fail'
};

export function needleBadge(result) {
  if (!result) return '-';
  const key = String(result).toUpperCase();
  const cls = NEEDLE_TAGS[key] || 'tag-warning';
  const label = key.replace(/_/g, ' ');
  return `<span class="tag ${cls}">${label}</span>`;
}
