<script lang="ts">
  import { untrack, onMount } from 'svelte';
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import type { Snapshot } from './model';
  let {
    status,
    send,
    onclose,
  }: {
    status: Snapshot;
    send: (op: string, data?: Record<string, unknown>) => Promise<unknown>;
    onclose: () => void;
  } = $props();
  let ssid = $state(untrack(() => status.board.ssid ?? '')),
    password = $state(''),
    busy = $state(false),
    error = $state('');
  let startAtLogin = $state(false);
  onMount(() => {
    if (isTauri())
      void invoke<boolean>('login_start')
        .then((v) => (startAtLogin = v))
        .catch((e) => (error = String(e)));
  });
  async function setLogin(enabled: boolean) {
    try {
      startAtLogin = await invoke<boolean>('login_start', { enabled });
    } catch (e) {
      error = String(e);
    }
  }
  async function connect() {
    busy = true;
    try {
      await send('wifi', { ssid, password });
      password = '';
      error = '';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function forget() {
    busy = true;
    try {
      await send('wifi_forget');
      ssid = '';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function scan() {
    try {
      await send('wifi_scan');
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="settings-panel">
  <div class="inspector-heading">
    <span class="eyebrow">BOARD SETTINGS</span><button
      class="icon-button"
      aria-label="Close board settings"
      onclick={onclose}>×</button
    >
  </div>
  <h1>Wi-Fi</h1>
  <p class="intro">Wi-Fi keeps the board’s clock in time, even when your computer is off.</p>
  <div class="wifi-status">
    <span class="small-dot" class:muted={!status.board.wifi_connected}></span>{status.board.wifi_connected
      ? `Connected to ${status.board.ssid}`
      : status.board.ssid
        ? `Not connected to ${status.board.ssid}`
        : 'Wi-Fi not connected'}
  </div>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void connect();
    }}
  >
    <div class="field">
      <div class="value-row">
        <label for="ssid">Network</label><button
          type="button"
          class="text-button"
          onclick={scan}
          disabled={!status.connected}>{status.board.scanning ? 'Scanning…' : 'Scan'}</button
        >
      </div>
      <input
        id="ssid"
        list="networks"
        placeholder="2.4 GHz Wi-Fi name"
        maxlength="32"
        bind:value={ssid}
        disabled={!status.connected}
      /><datalist id="networks"
        >{#each status.board.networks ?? [] as n}<option value={n.ssid}>{n.rssi} dBm</option>{/each}</datalist
      >
    </div>
    <div class="field">
      <label for="password">Password</label><input
        id="password"
        type="password"
        placeholder="Leave empty for an open network"
        maxlength="63"
        autocomplete="new-password"
        bind:value={password}
        disabled={!status.connected}
      />
    </div>
    <p class="hint">The ESP32 uses 2.4 GHz networks. Credentials are stored on this board.</p>
    <button class="primary full" type="submit" disabled={!status.connected || !ssid || busy}
      >Connect <span>↗</span></button
    >
  </form>
  {#if status.board.ssid}<button
      class="text-button forget"
      onclick={forget}
      disabled={busy || !status.connected}>Forget network</button
    >{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="settings-foot">
    <div class="field inline-field">
      <label for="login">Start at login</label><input
        id="login"
        class="switch"
        type="checkbox"
        checked={startAtLogin}
        onchange={(e) => void setLogin(e.currentTarget.checked)}
      />
    </div>
    <span class="eyebrow">DEVICE</span>
    <div class="device-line"><span>ESP Gauge</span><span class="mono">{status.device || '—'}</span></div>
    <p class="hint">
      Gauges and calibration travel with your board. The clock uses your computer’s current UTC offset.
    </p>
  </div>
</div>
