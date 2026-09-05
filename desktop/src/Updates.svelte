<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { isTauri } from '@tauri-apps/api/core';
  import type { Snapshot } from './model';
  import { updates, runUpdate } from './updateState.svelte';
  let { status, onprepare }: { status: Snapshot; onprepare: () => Promise<boolean> } = $props();
  let version = $state(''),
    target = $state(''),
    installing = $state(false);
  let path = $derived(
    target || (status.connected ? status.path : status.candidates.length === 1 ? status.candidates[0] : ''),
  );
  onMount(() => {
    if (isTauri()) void getVersion().then((v) => (version = v));
  });
  async function install(firmware: boolean) {
    installing = true;
    try {
      if (await onprepare())
        await runUpdate(
          firmware ? 'install_firmware' : 'install_app_update',
          firmware ? { path } : undefined,
        );
    } finally {
      installing = false;
    }
  }
</script>

<div class="updates-panel">
  <h1>Updates</h1>
  <div class="update-item">
    <div class="value-row"><span>ESP Gauge</span><span class="mono subtle">{version}</span></div>
    {#if updates.app_version}
      <p class="hint">Version {updates.app_version} is available.</p>
      <button class="primary full" disabled={updates.busy || installing} onclick={() => install(false)}
        >Update and restart <span>↗</span></button
      >
    {:else if updates.checked}<p class="hint">You’re up to date.</p>{/if}
  </div>
  <div class="update-item">
    <div class="value-row">
      <span>Board firmware</span><span class="mono subtle">{status.connected ? status.firmware : '—'}</span>
    </div>
    {#if updates.firmware_version}<p class="hint">Latest release · {updates.firmware_version}</p>{/if}
    {#if status.candidates.length > 0}
      {#if !status.connected || status.candidates.length > 1}
        <label class="hint" for="firmware-target">Install on</label>
        <select
          id="firmware-target"
          value={path}
          onchange={(e) => (target = e.currentTarget.value)}
          disabled={updates.busy || installing}
        >
          <option value="" disabled>Choose a USB board</option>
          {#each status.candidates as candidate}<option value={candidate}
              >{candidate}{status.devices.some((d) => d.path === candidate)
                ? ' · ESP Gauge'
                : ' · Unidentified CH340C'}</option
            >{/each}
        </select>
      {/if}
      {#if path && !status.devices.some((d) => d.path === path)}
        <p class="hint">
          This CH340C hasn’t identified as ESP Gauge. Install only if it’s your six-output board; its existing
          firmware will be replaced.
        </p>
      {/if}
      <button
        class="primary full"
        disabled={!path || !updates.firmware_version || updates.busy || installing}
        onclick={() => install(true)}
      >
        {status.connected && path === status.path
          ? status.firmware === updates.firmware_version
            ? 'Reinstall firmware'
            : 'Update firmware'
          : 'Install firmware'} <span>↗</span>
      </button>
      <p class="hint">Keep USB connected during installation. Your gauge settings stay on the board.</p>
    {:else}<p class="hint">Connect your board to install firmware.</p>{/if}
  </div>
  {#if updates.busy}
    <div class="update-progress" role="status">
      <span>{updates.stage}…</span><progress max="100" value={updates.progress}></progress>
    </div>
  {:else}<button class="text-button" onclick={() => runUpdate('check_updates')} disabled={installing}
      >Check for updates</button
    >{/if}
  {#if updates.error}<p class="error" role="alert">{updates.error}</p>{/if}
</div>
