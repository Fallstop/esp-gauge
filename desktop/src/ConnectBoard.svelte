<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';

  type DriverStatus = { supported: boolean; installed: boolean | null };
  let driver = $state<DriverStatus>({ supported: navigator.userAgent.includes('Windows'), installed: null });
  let busy = $state(false),
    error = $state(''),
    opened = $state(false);
  async function refresh() {
    try {
      driver = await invoke<DriverStatus>('usb_driver_status');
    } catch {
      // Keep the repair action available when detection fails.
    }
  }
  onMount(() => {
    if (isTauri()) void refresh();
  });
  async function install() {
    if (!isTauri()) return;
    busy = true;
    error = '';
    opened = false;
    try {
      await invoke('install_usb_driver');
      opened = true;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="connect-board">
  <h1>Connect your board</h1>
  <p>Plug your ESP Gauge into USB to begin.</p>
  {#if driver.supported}
    <button class="driver-setup-button" disabled={busy} onclick={install}>
      {busy ? 'USB driver setup is open…' : 'USB driver setup'}
      <span aria-hidden="true">↗</span>
    </button>
    <p class="driver-help" role="status">
      {#if busy}Approve the Windows prompt, then choose INSTALL in the WCH window.
      {:else if opened}If installation succeeded, reconnect your board. Restart Windows if prompted.
      {:else if driver.installed}USB driver detected. Open setup if your board still won’t connect.
      {:else}Board not found? Install the included USB driver. Administrator approval is required.{/if}
    </p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {/if}
</div>
