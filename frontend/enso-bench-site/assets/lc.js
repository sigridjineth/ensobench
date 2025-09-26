import { fetchJSON, fmtScore, needleBadge } from './utils.js';

const els = {
  siteTitle: document.getElementById('site-title'),
  navHome: document.getElementById('nav-home'),
  navTraj: document.getElementById('nav-trajectories'),
  navLc: document.getElementById('nav-lc'),
  title: document.getElementById('lc-title'),
  subtitle: document.getElementById('lc-subtitle'),
  coverageLabel: document.getElementById('lc-knowledge-label'),
  coveragePath: document.getElementById('lc-knowledge-path'),
  coverageDesc: document.getElementById('lc-knowledge-desc'),
  coverageCta: document.getElementById('lc-knowledge-cta'),
  needleLabel: document.getElementById('lc-hian-label'),
  needlePath: document.getElementById('lc-hian-path'),
  needleDesc: document.getElementById('lc-hian-desc'),
  needleCta: document.getElementById('lc-hian-cta'),
  tableCaption: document.getElementById('lc-table-caption'),
  colRank: document.getElementById('lc-col-rank'),
  colAgent: document.getElementById('lc-col-model'),
  colCoverage: document.getElementById('lc-col-core'),
  colIntent: document.getElementById('lc-col-knowledge'),
  colNeedleResult: document.getElementById('lc-col-hian-result'),
  colNeedleScore: document.getElementById('lc-col-hian-score'),
  colActions: document.getElementById('lc-col-programs'),
  colRuns: document.getElementById('lc-col-runs'),
  tbody: document.getElementById('lc-body'),
  notesTitle: document.getElementById('lc-notes-title'),
  notes: document.getElementById('lc-notes')
};

async function main() {
  const content = await fetchJSON('./data/content.json');
  const data = await fetchJSON('./data/models.json');
  const runs = [...(data.runs || [])].sort((a, b) => a.rank - b.rank);

  els.siteTitle.textContent = content.site.title;
  els.navHome.textContent = content.site.nav.home;
  els.navTraj.textContent = content.site.nav.trajectories;
  els.navLc.textContent = content.site.nav.needle;

  els.title.textContent = content.needle.title;
  els.subtitle.textContent = content.needle.subtitle;

  const coverageDataset = data.meta?.coverage_dataset || runs[0]?.intent_eval?.dataset || '-';
  const needleDataset = data.meta?.needle_dataset || runs[0]?.needle_eval?.dataset || '-';

  els.coverageLabel.textContent = content.needle.datasets.coverage.label;
  els.coveragePath.textContent = coverageDataset;
  els.coverageDesc.textContent = content.needle.datasets.coverage.description;
  els.coverageCta.textContent = content.needle.datasets.coverage.cta;
  const coverageHref = /^https?:/i.test(coverageDataset) ? coverageDataset : '#';
  els.coverageCta.href = coverageHref;
  els.coverageCta.target = coverageHref === '#' ? '_self' : '_blank';

  els.needleLabel.textContent = content.needle.datasets.needle.label;
  els.needlePath.textContent = needleDataset;
  els.needleDesc.textContent = content.needle.datasets.needle.description;
  els.needleCta.textContent = content.needle.datasets.needle.cta;
  const needleHref = /^https?:/i.test(needleDataset) ? needleDataset : '#';
  els.needleCta.href = needleHref;
  els.needleCta.target = needleHref === '#' ? '_self' : '_blank';

  els.tableCaption.textContent = content.needle.table.caption;
  els.colRank.textContent = content.needle.table.cols.rank;
  els.colAgent.textContent = content.needle.table.cols.agent;
  els.colCoverage.textContent = content.needle.table.cols.coverage;
  els.colIntent.textContent = content.needle.table.cols.intent;
  els.colNeedleResult.textContent = content.needle.table.cols.needle_result;
  els.colNeedleScore.textContent = content.needle.table.cols.needle_score;
  els.colActions.textContent = content.needle.table.cols.actions;
  els.colRuns.textContent = content.needle.table.cols.runs;

  els.tbody.innerHTML = runs
    .map(run => {
      const needleLink = run.needle_eval
        ? ` · <a class="underline" href="./trajectories.html#needle-${run.needle_eval.run_id}">Needle</a>`
        : '';
      const uniqueActions = run.coverage_score?.unique_actions ?? '-';
      return `
        <tr class="border-b last:border-0">
          <td class="px-3 py-2">${run.rank}</td>
          <td class="px-3 py-2">${run.agent}</td>
          <td class="px-3 py-2 text-right font-mono">${fmtScore(run.coverage_score?.median)}</td>
          <td class="px-3 py-2 text-right font-mono">${fmtScore(run.intent_eval?.score)}</td>
          <td class="px-3 py-2">${needleBadge(run.needle_eval?.result)}</td>
          <td class="px-3 py-2 text-right font-mono">${run.needle_eval ? fmtScore(run.needle_eval.score) : '-'}</td>
          <td class="px-3 py-2 text-right">${uniqueActions}</td>
          <td class="px-3 py-2 text-slate-700 text-xs">
            <a class="underline" href="./trajectories.html#coverage-${run.coverage_run_id}">Coverage</a>${needleLink}
          </td>
        </tr>
      `;
    })
    .join('');

  els.notesTitle.textContent = content.needle.notes.title;
  els.notes.innerHTML = content.needle.notes.items.map(item => `<li>${item}</li>`).join('');
}

main().catch(err => console.error(err));
