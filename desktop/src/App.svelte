<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { emptyConfig, emptySnapshot, type Config, type Channel, type Snapshot } from './model';
  import Board from './Board.svelte';
  import Inspector from './Inspector.svelte';
  import Calibration from './Calibration.svelte';
  import Settings from './Settings.svelte';
  import { startUpdates, updates, newerVersion } from './updateState.svelte';
  let status = $state<Snapshot>(emptySnapshot()),
    config = $state<Config>(emptyConfig());
  let selected = $state(0),
    calibrating = $state(false),
    settings = $state(false),
    error = $state(''),
    pending = $state(false),
    configDevice = '';
  let saving = false,
    revision = 0,
    timer: ReturnType<typeof setTimeout> | undefined;
  let saveTask: Promise<boolean> | null = null;
  let installing = $derived(updates.busy && updates.stage !== 'Checking releases');
  async function send(op: string, data: Record<string, unknown> = {}) {
    return invoke('command', { command: { op, device: status.device, ...data } });
  }
  function accept(next: Snapshot) {
    const reconnected = next.connected && !status.connected && pending && next.device === configDevice;
    const changed = next.device && configDevice && next.device !== configDevice;
    if (changed) {
      pending = false;
      revision++;
      calibrating = false;
      clearTimeout(timer);
    }
    status = next;
    if (next.config && !pending && !saving) {
      if (JSON.stringify($state.snapshot(config)) !== JSON.stringify(next.config))
        config = structuredClone(next.config);
      configDevice = next.device;
    }
    if (!next.connected) calibrating = false;
    if (reconnected) timer = setTimeout(() => void save(), 0);
  }
  async function save(): Promise<boolean> {
    if (saveTask) {
      if (!(await saveTask)) return false;
      return save();
    }
    if (!pending) return true;
    if (!status.connected || status.device !== configDevice) {
      error = 'Reconnect this board to finish applying your changes.';
      return false;
    }
    saving = true;
    const version = revision;
    saveTask = (async () => {
      try {
        await send('config', { config: structuredClone($state.snapshot(config)) });
        if (version === revision) pending = false;
        error = '';
        return true;
      } catch (e) {
        error = String(e);
        return false;
      } finally {
        saving = false;
        saveTask = null;
        if (pending && version !== revision) timer = setTimeout(() => void save(), 150);
      }
    })();
    return saveTask;
  }
  function change(patch: Partial<Channel>) {
    if (installing) return;
    config.channels[selected] = { ...config.channels[selected], ...patch };
    pending = true;
    configDevice = status.device;
    revision++;
    clearTimeout(timer);
    timer = setTimeout(() => void save(), 220);
  }
  async function calibrate() {
    if (installing) return;
    error = '';
    clearTimeout(timer);
    if (pending && !(await save())) return;
    try {
      await send('calibrate', { port: selected, duty: 0 });
      calibrating = true;
      settings = false;
    } catch (e) {
      error = String(e);
    }
  }
  async function cancel() {
    try {
      await send('calibrate_end');
    } catch (e) {
      error = String(e);
    }
    calibrating = false;
  }
  async function finish(min: number, max: number) {
    change({ enabled: true, min_duty: min, max_duty: max });
    clearTimeout(timer);
    if (!(await save())) return false;
    await cancel();
    return true;
  }
  async function select(n: number) {
    if (installing) return;
    if (calibrating) await cancel();
    selected = n;
    settings = false;
  }
  async function pause() {
    try {
      await send('pause', { paused: !status.paused });
      if (calibrating) calibrating = false;
    } catch (e) {
      error = String(e);
    }
  }
  async function retry() {
    try {
      await send('retry');
      if (pending) await save();
      error = '';
    } catch (e) {
      error = String(e);
    }
  }
  async function boardSettings() {
    if (calibrating) await cancel();
    settings = !settings;
    if (settings && status.connected)
      try {
        await send('wifi_scan');
      } catch (e) {
        error = String(e);
      }
  }
  async function prepareUpdate() {
    commitEditing();
    clearTimeout(timer);
    if (calibrating) await cancel();
    return !pending || (await save());
  }
  function commitEditing() {
    const input = document.activeElement;
    if (input instanceof HTMLInputElement) {
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
      input.blur();
    }
  }
  onMount(() => {
    if (!isTauri()) return;
    const stopUpdates = startUpdates();
    let unlisten: () => void = () => {};
    let unlistenQuit: () => void = () => {};
    let disposed = false;
    void listen<Snapshot>('state', (event) => accept(event.payload)).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    void invoke<Snapshot>('snapshot').then(accept);
    void listen('quit-requested', async () => {
      // Native menu shortcuts can arrive before the last text input is committed.
      await new Promise((resolve) => setTimeout(resolve, 80));
      commitEditing();
      clearTimeout(timer);
      if (calibrating) await cancel();
      if (pending && !(await save())) {
        await invoke('show_window');
        return;
      }
      await invoke('quit');
    }).then((fn) => {
      if (disposed) fn();
      else unlistenQuit = fn;
    });
    const visible = () => {
      if (document.hidden) {
        commitEditing();
        if (calibrating) void cancel();
        if (pending) {
          clearTimeout(timer);
          void save();
        }
      } else if (!document.hidden) void invoke<Snapshot>('snapshot').then(accept);
    };
    document.addEventListener('visibilitychange', visible);
    return () => {
      disposed = true;
      stopUpdates();
      unlisten();
      unlistenQuit();
      clearTimeout(timer);
      document.removeEventListener('visibilitychange', visible);
    };
  });
  let enabled = $derived(config.channels.filter((c) => c.enabled).length);
</script>

<div
  class="app-shell"
  class:disconnected={!status.connected}
  class:settings-open={settings}
  class:macos={navigator.userAgent.includes('Mac')}
>
  <header class="app-header" data-tauri-drag-region>
    <div class="wordmark" data-tauri-drag-region>
      <svg viewBox="0 0 32 32" aria-hidden="true"
        ><path d="M4 23a13 13 0 0 1 24 0M16 23l8-13" /><circle cx="16" cy="23" r="2" /></svg
      ><span data-tauri-drag-region>ESP <b data-tauri-drag-region>GAUGE</b></span>
    </div>
    <div class="header-actions">
      {#if status.devices.length > 1}<select
          class="board-select"
          aria-label="Connected board"
          disabled={installing}
          value={status.path}
          onchange={async (e) => {
            if (calibrating) await cancel();
            try {
              await send('select', { path: e.currentTarget.value });
            } catch (e) {
              error = String(e);
            }
          }}
          >{#each status.devices as d}<option value={d.path}>Board {d.id.slice(-6)}</option>{/each}</select
        >{:else}<div class="connection">
          <span class="small-dot" class:muted={!status.connected}></span>{status.connected
            ? 'USB connected'
            : 'Looking for your board'}
        </div>{/if}
      <div class="header-divider"></div>
      <button
        class="icon-button"
        aria-label={status.paused ? 'Resume gauges' : 'Pause gauges'}
        title={status.paused ? 'Resume gauges' : 'Pause gauges'}
        onclick={pause}
        disabled={!status.connected || !enabled || installing}
        ><svg viewBox="0 0 24 24" aria-hidden="true"
          >{#if status.paused}<path d="m8 5 10 7-10 7Z" />{:else}<path d="M8 5v14M16 5v14" />{/if}</svg
        ></button
      >
      <button
        class="icon-button"
        class:active={settings}
        class:has-update={!!updates.app_version ||
          !!(status.connected && newerVersion(updates.firmware_version, status.firmware))}
        aria-label="Board settings"
        title="Board settings"
        onclick={boardSettings}
        ><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M4 17h16M8 4v6M16 14v6" /></svg></button
      >
    </div>
  </header>
  <main>
    <section class="board-panel" aria-label="Gauge outputs" inert={installing}>
      <Board {config} {status} {selected} onselect={(n) => void select(n)} />
      <div class="disconnected-message" aria-live="polite">
        {!status.connected ? 'Connect your board to begin.' : ''}
      </div>
      <div class="board-panel-foot" inert={!status.connected}>
        <span
          >{calibrating
            ? `Adjusting PWM${selected + 1}`
            : enabled
              ? 'Select a header to change its gauge.'
              : 'Select a header to add your first gauge.'}</span
        >
      </div>
      <div class="computer-strip" inert={!status.connected}>
        <div>
          <span>CPU</span><strong
            >{status.metrics.cpu == null ? '—' : status.metrics.cpu.toFixed(0)}<small>%</small></strong
          >
          <div class="metric-track"><i style:width="{status.metrics.cpu ?? 0}%"></i></div>
        </div>
        <div>
          <span>MEMORY</span><strong
            >{status.metrics.memory == null ? '—' : status.metrics.memory.toFixed(0)}<small>%</small></strong
          >
          <div class="metric-track"><i style:width="{status.metrics.memory ?? 0}%"></i></div>
        </div>
        <div>
          <span>NETWORK ↓</span><strong
            >{status.metrics.network_down == null ? '—' : status.metrics.network_down.toFixed(2)}<small
              >MiB/s</small
            ></strong
          >
          <div class="metric-track">
            <i style:width="{Math.min(100, (status.metrics.network_down ?? 0) * 10)}%"></i>
          </div>
        </div>
      </div>
    </section>
    <aside
      class="inspector"
      inert={(!status.connected || installing) && !settings}
      aria-label={settings ? 'Board settings' : `PWM${selected + 1} settings`}
    >
      {#if settings}<Settings {status} {send} onprepare={prepareUpdate} onclose={() => (settings = false)} />
      {:else if calibrating}<Calibration
          port={selected}
          channel={config.channels[selected]}
          {send}
          onfinish={finish}
          oncancel={cancel}
        />
      {:else}<Inspector
          channel={config.channels[selected]}
          port={selected}
          {status}
          onchange={change}
          oncalibrate={() => void calibrate()}
          onremove={() => change({ enabled: false })}
        />{/if}
    </aside>
  </main>
  {#if error || status.error}<div class="error-bar" role="alert">
      <span>{error || status.error}</span><button onclick={retry}>Retry</button><button
        aria-label="Dismiss error"
        onclick={() => {
          error = '';
          status.error = null;
        }}>×</button
      >
    </div>{/if}
  {#if !isTauri()}<div class="preview-notice">
      Design preview · open the desktop app to connect hardware.
    </div>{/if}
</div>
