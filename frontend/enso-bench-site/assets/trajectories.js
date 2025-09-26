import { fetchJSON, readFileAsText, parseJSONL, domainPill, actionLabel, fmtScore, needleBadge } from './utils.js';

const els = {
  siteTitle: document.getElementById('site-title'),
  navHome: document.getElementById('nav-home'),
  navTraj: document.getElementById('nav-trajectories'),
  navLc: document.getElementById('nav-lc'),
  title: document.getElementById('traj-title'),
  subtitle: document.getElementById('traj-subtitle'),
  loadSample: document.getElementById('load-sample'),
  or: document.getElementById('traj-or'),
  pickLabel: document.getElementById('traj-pick-label'),
  note: document.getElementById('traj-note'),
  pickRun: document.getElementById('pick-run'),
  txBody: document.getElementById('tx-body'),
  dlg: document.getElementById('dlg'),
  dlgContent: document.getElementById('dlg-content'),
  dlgTitle: document.getElementById('traj-detail-title'),
  dlgClose: document.getElementById('traj-close'),
  runInfo: document.getElementById('run-info'),
  meta: document.getElementById('meta'),
  select: document.getElementById('traj-select'),
  selectLabel: document.getElementById('traj-select-label')
};

let runOptions = [];

async function main() {
  const content = await fetchJSON('./data/content.json');
  const data = await fetchJSON('./data/models.json');
  const runs = [...(data.runs || [])].sort((a, b) => a.rank - b.rank);

  els.siteTitle.textContent = content.site.title;
  els.navHome.textContent = content.site.nav.home;
  els.navTraj.textContent = content.site.nav.trajectories;
  els.navLc.textContent = content.site.nav.needle;

  els.title.textContent = content.trajectories.title;
  els.subtitle.textContent = content.trajectories.subtitle;
  els.loadSample.textContent = content.trajectories.uploader.sample_btn;
  els.or.textContent = content.trajectories.uploader.or;
  els.pickLabel.textContent = content.trajectories.uploader.pick_label;
  els.note.textContent = content.trajectories.uploader.note;
  els.dlgTitle.textContent = content.trajectories.detail_title;
  els.dlgClose.textContent = content.trajectories.close;
  els.selectLabel.textContent = content.trajectories.uploader.select_label;

  runOptions = runs.flatMap(run => {
    const options = [
      {
        id: `coverage-${run.coverage_run_id}`,
        type: 'coverage',
        path: run.coverage_run,
        runId: run.coverage_run_id,
        agent: run.agent,
        rank: run.rank,
        label: `#${run.rank} · ${run.agent} · Coverage`,
        coverageScore: run.coverage_score,
        needle: run.needle_eval
      }
    ];
    if (run.needle_eval?.run_dir) {
      options.push({
        id: `needle-${run.needle_eval.run_id}`,
        type: 'needle',
        path: run.needle_eval.run_dir,
        runId: run.needle_eval.run_id,
        agent: run.agent,
        rank: run.rank,
        label: `#${run.rank} · ${run.agent} · Needle (${run.needle_eval.result})`,
        coverageScore: run.coverage_score,
        needle: run.needle_eval
      });
    }
    return options;
  });

  if (!runOptions.length) {
    els.select.innerHTML = '<option value="">No leaderboard runs found</option>';
  } else {
    els.select.innerHTML = runOptions.map(opt => `<option value="${opt.id}">${opt.label}</option>`).join('');
  }

  els.loadSample.addEventListener('click', async () => {
    const opt = runOptions.find(o => o.id === els.select.value);
    if (opt) {
      await loadFromHosted(opt);
      window.location.hash = opt.id;
    }
  });

  els.pickRun.addEventListener('change', async (e) => {
    const files = Array.from(e.target.files || []);
    const map = Object.fromEntries(
      files.map(f => [f.webkitRelativePath.split('/').pop(), f])
    );
    if (!map['eval_per_tx.jsonl'] || !map['per_tx.jsonl']) {
      alert(content.trajectories.errors?.missing_files || 'Missing required files.');
      return;
    }
    const evalText = await readFileAsText(map['eval_per_tx.jsonl']);
    const perText = await readFileAsText(map['per_tx.jsonl']);
    const score = map['eval_score.json']
      ? JSON.parse(await readFileAsText(map['eval_score.json']))
      : null;
    const needleEval = map['eval_needle.json']
      ? JSON.parse(await readFileAsText(map['eval_needle.json']))
      : null;
    render(null, evalText, perText, { score, meta: null, needleEval });
    window.location.hash = '';
  });

  const initialHash = window.location.hash.replace('#', '');
  const initialOption = runOptions.find(opt => opt.id === initialHash) || runOptions[0] || null;
  if (initialOption) {
    els.select.value = initialOption.id;
    await loadFromHosted(initialOption);
    window.location.hash = initialOption.id;
  }
}

async function loadFromHosted(option) {
  if (!option) return;
  const base = option.path;
  const [evalText, perText] = await Promise.all([
    fetch(base + '/eval_per_tx.jsonl').then(r => r.text()),
    fetch(base + '/per_tx.jsonl').then(r => r.text())
  ]);
  const [score, meta, needleEval] = await Promise.all([
    fetchJSON(base + '/eval_score.json').catch(() => null),
    fetchJSON(base + '/meta.json').catch(() => null),
    option.type === 'needle'
      ? fetchJSON(base + '/eval_needle.json').catch(() => option.needle || null)
      : Promise.resolve(option.needle || null)
  ]);
  render(option, evalText, perText, { score, meta, needleEval });
}

function render(option, evalText, perText, extra = {}) {
  const summaries = parseJSONL(evalText);
  const perTx = parseJSONL(perText);
  const rawByDigest = new Map(perTx.map(v => [v.intent_id || v.digest, v]));
  const { score, meta, needleEval } = extra;

  if (option || score || needleEval) {
    const parts = [];
    if (option) {
      parts.push(`#${option.rank} · ${option.agent}`);
      const result = option.type === 'needle'
        ? (option.needle?.result || needleEval?.result || '')
        : '';
      const runLabel = option.type === 'coverage' ? 'Coverage run' : 'Needle run';
      parts.push(result ? `${runLabel} ${needleBadge(result)}` : runLabel);
      if (option.coverageScore?.median != null) {
        parts.push(`Coverage median ${fmtScore(option.coverageScore.median)}`);
      }
      if (option.needle?.score != null) {
        parts.push(`Needle ${fmtScore(option.needle.score)}`);
      }
    } else {
      parts.push('Uploaded run');
    }
    if (score?.final_score != null) parts.push(`Final score ${fmtScore(score.final_score)}`);
    if (score?.bonus != null) parts.push(`Bonus ${fmtScore(score.bonus)}`);
    if (needleEval?.score != null && !parts.some(p => p.startsWith('Needle'))) {
      parts.push(`Needle ${fmtScore(needleEval.score)}`);
    }
    if (meta?.bench_version) parts.push(`Bench ${meta.bench_version}`);
    els.meta.innerHTML = parts.join(' · ');
    els.runInfo.classList.remove('hidden');
  } else {
    els.runInfo.classList.add('hidden');
  }

  els.txBody.innerHTML = summaries
    .map(s => {
      const digest = s.intent_id || s.digest;
      const domains = Array.isArray(s.domains) ? s.domains : [];
      const actions = Array.isArray(s.actions) ? s.actions : [];
      return `
        <tr class="border-b last:border-0 align-top">
          <td class="px-3 py-2 font-mono text-xs">${digest}</td>
          <td class="px-3 py-2">${domains.map(domainPill).join(' ') || '-'}</td>
          <td class="px-3 py-2 text-xs leading-5">${actions.map(actionLabel).join('<br/>') || '-'}</td>
          <td class="px-3 py-2 text-right">${fmtScore(s.bonus)}</td>
          <td class="px-3 py-2 text-right">${fmtScore(s.penalty)}</td>
          <td class="px-3 py-2 text-right">
            <button data-digest="${digest}" class="text-xs px-2 py-1 rounded border">Detail</button>
          </td>
        </tr>
      `;
    })
    .join('');

  els.txBody.querySelectorAll('button[data-digest]').forEach(btn => {
    btn.addEventListener('click', () => {
      const dg = btn.getAttribute('data-digest');
      const summary = summaries.find(x => (x.intent_id || x.digest) === dg);
      const raw = rawByDigest.get(dg);
      const detail = {
        summary,
        raw,
        needle: needleEval,
        meta: option ? { id: option.id, agent: option.agent, type: option.type } : undefined
      };
      els.dlgContent.textContent = JSON.stringify(detail, null, 2);
      els.dlg.showModal();
    });
  });
}

main().catch(err => console.error(err));
