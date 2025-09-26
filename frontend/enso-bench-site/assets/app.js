import { fetchJSON, fmtScore, needleBadge } from './utils.js';

const els = {
  siteTitle: document.getElementById('site-title'),
  navHome: document.getElementById('nav-home'),
  navTraj: document.getElementById('nav-trajectories'),
  navLc: document.getElementById('nav-lc'),
  heroTitle: document.getElementById('hero-title'),
  heroSubtitle: document.getElementById('hero-subtitle'),
  pillars: document.getElementById('pillars'),
  scoreboardTitle: document.getElementById('scoreboard-title'),
  scoreboardSource: document.getElementById('scoreboard-source'),
  scoreboardLink: document.getElementById('scoreboard-link'),
  chartScoreTitle: document.getElementById('chart-score-title'),
  chartDomainTitle: document.getElementById('chart-domain-title'),
  howtoTitle: document.getElementById('howto-title'),
  howtoSteps: document.getElementById('howto-steps'),
  howtoFoot: document.getElementById('howto-foot'),
  footerText: document.getElementById('footer-text'),
  lbBody: document.getElementById('lb-body'),
  scoreChart: document.getElementById('scoreChart'),
  domainChart: document.getElementById('domainChart')
};

async function main() {
  const content = await fetchJSON('./data/content.json');
  const data = await fetchJSON('./data/models.json');
  const runs = [...(data.runs || [])].sort((a, b) => a.rank - b.rank);

  els.siteTitle.textContent = content.site.title;
  els.navHome.textContent = content.site.nav.home;
  els.navTraj.textContent = content.site.nav.trajectories;
  els.navLc.textContent = content.site.nav.needle;

  els.heroTitle.textContent = content.home.hero.title;
  els.heroSubtitle.textContent = content.home.hero.subtitle;

  els.pillars.innerHTML = content.home.hero.pillars
    .map(p => `
      <div class="p-4 bg-white rounded-lg shadow-sm border">
        <h3 class="font-semibold mb-1">${p.title}</h3>
        <p class="text-sm text-slate-600">${p.text}</p>
      </div>
    `)
    .join('');

  els.scoreboardTitle.textContent = content.home.scoreboard.title;
  const coverageDataset = data.meta?.coverage_dataset || runs[0]?.coverage_score?.dataset || '-';
  const needleDataset = data.meta?.needle_dataset || runs[0]?.needle_eval?.dataset || '-';
  els.scoreboardSource.textContent = `${content.home.scoreboard.source} ${coverageDataset} · ${needleDataset}`;
  els.scoreboardLink.textContent = content.home.scoreboard.link_text;
  els.scoreboardLink.href = content.home.scoreboard.link_href || './trajectories.html';

  els.chartScoreTitle.textContent = content.home.charts.coverage;
  els.chartDomainTitle.textContent = content.home.charts.needle;

  els.howtoTitle.textContent = content.home.howto.title;
  els.howtoSteps.innerHTML = content.home.howto.steps.map(step => `<li>${step}</li>`).join('');
  els.howtoFoot.textContent = content.home.howto.footnote;
  els.footerText.textContent = content.home.footer;

  els.lbBody.innerHTML = runs
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

  const labels = runs.map(run => run.agent);
  const coverageScores = runs.map(run => run.coverage_score?.median ?? 0);
  const intentScores = runs.map(run => run.intent_eval?.score ?? 0);
  const needleScores = runs.map(run => run.needle_eval?.score ?? 0);

  new Chart(els.scoreChart, {
    type: 'bar',
    data: {
      labels,
      datasets: [
        {
          label: 'Coverage median',
          data: coverageScores,
          backgroundColor: '#0f172a'
        },
        {
          label: 'Intent score',
          data: intentScores,
          backgroundColor: '#34d399'
        }
      ]
    },
    options: {
      plugins: { legend: { position: 'bottom' } },
      scales: { y: { beginAtZero: true } }
    }
  });

  new Chart(els.domainChart, {
    type: 'bar',
    data: {
      labels,
      datasets: [
        {
          label: 'Needle score',
          data: needleScores,
          backgroundColor: '#f97316'
        }
      ]
    },
    options: {
      plugins: { legend: { display: false } },
      scales: { y: { beginAtZero: true, suggestedMax: 5 } }
    }
  });
}

main().catch(err => console.error(err));
