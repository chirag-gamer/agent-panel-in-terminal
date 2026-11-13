import { invoke } from '@tauri-apps/api/core';
import type { AgentEntry, ApplyResult, Detection, PanelConfig } from './types';
import { effortLevels } from './types';

// ---------- state ----------
let detections: Detection[] = [];
let cfg: PanelConfig = { version: 1, agents: [], customize_commandline: null };
const modelsCache = new Map<string, string[]>();
/** UI-side effort selection per active agent (effort is embedded in the
 *  commandline by the Rust adapter, not persisted as a schema field). */
const effortSel = new Map<string, string>();

const $ = <T extends HTMLElement>(sel: string) => document.querySelector<T>(sel)!;

// ---------- helpers ----------
function logoImg(id: string, cls: string): HTMLImageElement {
  const img = document.createElement('img');
  img.className = cls;
  img.alt = '';
  let step = 0;
  const sources = [`logos/${id}.svg`, `logos/${id}.png`];
  img.onerror = () => {
    step += 1;
    if (step < sources.length) img.src = sources[step];
    else {
      img.onerror = null;
      img.src = 'data:image/svg+xml,' +
        encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" rx="2" fill="#8A8A8A"/></svg>');
    }
  };
  img.src = sources[0];
  return img;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls?: string, text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

function activeAgents(): AgentEntry[] {
  return [...cfg.agents].sort((a, b) => a.entry_order - b.entry_order);
}

function adapterById(id: string) {
  return detections.find((d) => d.adapter.id === id)?.adapter;
}

function setStatus(msg: string) {
  $('#status').textContent = msg;
}

/** Recompose a non-manual entry from the adapter + current selections. */
async function recompose(agent: AgentEntry, model: string | null, effort: string | null) {
  const composed = await invoke<AgentEntry>('compose_entry', {
    adapterId: agent.id,
    model: model || null,
    effort: effort || null,
  });
  agent.commandline = composed.commandline;
  agent.open_here_input = composed.open_here_input;
  agent.model = model || null;
}

// ---------- rendering ----------
function renderActive() {
  const wrap = $('#active-list');
  wrap.innerHTML = '';
  const agents = activeAgents();

  agents.forEach((agent, idx) => {
    const row = el('div', 'card active-card');
    if (!agent.enabled) row.classList.add('off');

    const top = el('div', 'row');
    top.append(el('span', 'pill', `#${idx + 1}`));
    if (!agent.manual) top.append(logoImg(agent.id, 'logo'));

    const name = el('span', 'name', agent.name);
    top.append(name);

    const sp = el('span', 'spacer');

    if (!agent.manual) {
      const left = el('button', 'icon-btn', '◀');
      left.title = 'Move up';
      left.onclick = () => move(agent, -1);
      const right = el('button', 'icon-btn', '▶');
      right.title = 'Move down';
      right.onclick = () => move(agent, 1);
      sp.append(left, right);
    }

    const onoff = el('button', 'icon-btn', agent.enabled ? 'On' : 'Off');
    onoff.onclick = () => {
      agent.enabled = !agent.enabled;
      renderAll();
    };
    const rm = el('button', 'btn ghost', 'Remove');
    rm.onclick = () => {
      cfg.agents = cfg.agents.filter((a) => a !== agent);
      renormalizeOrders();
      renderAll();
    };
    sp.append(onoff, rm);
    top.append(sp);
    row.append(top);

    // model / effort / refresh (built-in agents only)
    const adapter = adapterById(agent.id);
    if (!agent.manual && adapter) {
      const mid = el('div', 'row config-row');

      const sel = el('select', 'select') as HTMLSelectElement;
      sel.append(el('option', undefined, '(default model)'));
      const models = modelsCache.get(agent.id) ?? [];
      for (const m of models) {
        const o = el('option', undefined, m) as HTMLOptionElement;
        o.value = m;
        if (agent.model === m) o.selected = true;
      }
      if (agent.model && !models.includes(agent.model)) {
        const o = el('option', undefined, agent.model) as HTMLOptionElement;
        o.value = agent.model;
        o.selected = true;
      }
      sel.onchange = async () => {
        await recompose(agent, sel.value || null, effortSel.get(agent.id) ?? null);
        renderAll();
      };

      const manual = el('input', 'input') as HTMLInputElement;
      manual.type = 'text';
      manual.placeholder = 'manual model override…';
      manual.value = agent.model ?? '';
      manual.onchange = async () => {
        await recompose(agent, manual.value.trim() || null, effortSel.get(agent.id) ?? null);
        renderAll();
      };

      const { levels } = effortLevels(adapter.effort);
      let effSel: HTMLSelectElement | null = null;
      if (levels.length) {
        effSel = el('select', 'select small') as HTMLSelectElement;
        effSel.append(el('option', undefined, 'effort: (none)'));
        for (const lv of levels) {
          const o = el('option', undefined, lv) as HTMLOptionElement;
          o.value = lv;
          if (effortSel.get(agent.id) === lv) o.selected = true;
        }
        effSel.onchange = async () => {
          effortSel.set(agent.id, effSel!.value);
          await recompose(agent, agent.model, effSel!.value || null);
          renderAll();
        };
      }

      const refresh = el('button', 'btn ghost', 'Refresh models');
      refresh.onclick = async () => {
        refresh.disabled = true;
        try {
          const models = await invoke<string[]>('discover_models', { adapterId: agent.id });
          modelsCache.set(agent.id, models);
          setStatus(`Discovered ${models.length} model(s) for ${agent.name}.`);
        } catch (e) {
          setStatus(`Model discovery failed: ${e}`);
        }
        renderAll();
      };

      mid.append(sel, manual);
      if (effSel) mid.append(effSel);
      mid.append(refresh);
      row.append(mid);
    }

    const preview = el('div', 'chip', agent.commandline || '(no command)');
    row.append(preview);
    wrap.append(row);
  });

  // empty-slot placeholders up to the 4-slot maximum
  for (let i = agents.length; i < 4; i++) {
    const ph = el('div', 'card placeholder');
    ph.textContent = `Empty slot #${i + 1} — add from Available below`;
    wrap.append(ph);
  }
}

function renderAvailable() {
  const wrap = $('#available-list');
  wrap.innerHTML = '';
  const activeIds = new Set(cfg.agents.map((a) => a.id));
  const available = detections.filter((d) => d.installed && !activeIds.has(d.adapter.id));

  if (available.length === 0) {
    wrap.append(el('div', 'empty-note', 'All installed agent CLIs are already in the panel.'));
  }
  for (const d of available) {
    const row = el('div', 'card available-card');
    row.append(logoImg(d.adapter.id, 'logo'));
    const txt = el('div', 'avail-text');
    txt.append(el('div', 'name', d.adapter.name));
    txt.append(el('div', 'muted', d.version || d.adapter.exe));
    row.append(txt);
    const add = el('button', 'btn accent', 'Add');
    add.onclick = async () => {
      const entry = await invoke<AgentEntry>('compose_entry', {
        adapterId: d.adapter.id,
        model: null,
        effort: null,
      });
      entry.entry_order = cfg.agents.length;
      cfg.agents.push(entry);
      renderAll();
    };
    row.append(add);
    wrap.append(row);
  }
}

function renderManualForm() {
  const form = $('#manual-form');
  form.innerHTML = '';
  const name = el('input', 'input') as HTMLInputElement;
  name.placeholder = 'Agent name';
  const cmd = el('input', 'input wide') as HTMLInputElement;
  cmd.placeholder = 'launch command, e.g. myagent --model x';
  const add = el('button', 'btn accent', 'Add');
  const cancel = el('button', 'btn ghost', 'Cancel');
  cancel.onclick = () => {
    form.innerHTML = '';
  };
  add.onclick = () => {
    const c = cmd.value.trim();
    const n = name.value.trim();
    if (!n || !c) return;
    cfg.agents.push({
      id: `manual-${n.toLowerCase().replace(/[^a-z0-9-]+/g, '-')}`,
      name: n,
      commandline: c,
      open_here_input: `${c}\r`,
      model: null,
      entry_order: cfg.agents.length,
      enabled: true,
      manual: true,
    });
    form.innerHTML = '';
    renderAll();
  };
  form.append(name, cmd, add, cancel);
}

function renormalizeOrders() {
  activeAgents().forEach((a, i) => {
    a.entry_order = i;
  });
}

function move(agent: AgentEntry, dir: number) {
  const agents = activeAgents();
  const i = agents.indexOf(agent);
  const j = i + dir;
  if (j < 0 || j >= agents.length) return;
  [agents[i], agents[j]] = [agents[j], agents[i]];
  agents.forEach((a, k) => {
    a.entry_order = k;
  });
  renderAll();
}

function renderAll() {
  renderActive();
  renderAvailable();
}

// ---------- footer actions ----------
async function onApply() {
  try {
    await invoke('save_config', { cfg });
    const r = await invoke<ApplyResult>('apply_config', { cfg });
    setStatus(`Applied ✓ backup: ${r.backup_path} · backup SHA-256: ${r.backup_sha256} · new SHA-256: ${r.new_sha256}`);
  } catch (e) {
    setStatus(`Apply failed: ${e}`);
  }
}

async function onRestore() {
  try {
    const hash = await invoke<string>('restore_backup');
    setStatus(`Restored latest backup ✓ SHA-256: ${hash}`);
  } catch (e) {
    setStatus(`Restore failed: ${e}`);
  }
}

// ---------- init ----------
async function init() {
  const savedTheme = localStorage.getItem('ap-theme');
  if (savedTheme === 'light') document.documentElement.classList.add('light');
  $('#theme-toggle').onclick = () => {
    document.documentElement.classList.toggle('light');
    localStorage.setItem('ap-theme', document.documentElement.classList.contains('light') ? 'light' : 'dark');
  };
  $('#apply').onclick = onApply;
  $('#restore').onclick = onRestore;
  $('#quit').onclick = () => invoke('quit_app');
  $('#add-manual').onclick = () => {
    const form = $('#manual-form');
    if (form.innerHTML === '') renderManualForm();
    else form.innerHTML = '';
  };

  detections = await invoke<Detection[]>('detect_agents');
  cfg = await invoke<PanelConfig>('load_config');
  renormalizeOrders();
  renderAll();
  setStatus('Ready.');
}

init().catch((e) => setStatus(`Init failed: ${e}`));
