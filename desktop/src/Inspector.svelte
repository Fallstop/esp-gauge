<script lang="ts">
  import { sources, sourceFor, reading, type Channel, type Snapshot } from './model';
  import Dial from './Dial.svelte';
  import SourcePicker from './SourcePicker.svelte';
  let {
    channel,
    port,
    status,
    onchange,
    oncalibrate,
    onremove,
  }: {
    channel: Channel;
    port: number;
    status: Snapshot;
    onchange: (patch: Partial<Channel>) => void;
    oncalibrate: () => void;
    onremove: () => void;
  } = $props();
  let source = $derived(sourceFor(channel.source, status)),
    value = $derived(
      channel.source.startsWith('wave_')
        ? status.board.available?.[port]
          ? (status.board.positions?.[port] ?? 0) * 100
          : undefined
        : reading(channel, status),
    );
  let position = $derived(
    status.connected && status.board.available?.[port]
      ? channel.reverse
        ? 1 - (status.board.positions?.[port] ?? 0)
        : (status.board.positions?.[port] ?? 0)
      : 0,
  );
  let clock = $derived(source.group === 'Clock');
  let now = $state(new Date());
  import { onMount } from 'svelte';
  onMount(() => {
    const t = setInterval(() => (now = new Date()), 1000);
    return () => clearInterval(t);
  });
  let clockText = $derived(
    now.toLocaleTimeString([], {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      ...(channel.source === 'time_seconds' ? { second: '2-digit' } : {}),
    }),
  );
</script>

<div class="inspector-heading">
  <span class="eyebrow">PWM{port + 1}</span>{#if channel.enabled}<span
      class="small-dot"
      class:muted={!status.connected || status.paused}
    ></span>{/if}
</div>
{#if !channel.enabled}
  <div class="empty-inspector">
    <svg class="connector-icon" viewBox="0 0 100 100" aria-hidden="true"
      ><path
        d="m18 27 38-20 27 17v48L46 93 18 76Z M18 27l28 19 37-22M46 46v47M30 31l25-13 16 9-25 14Z M40 64V48m18 9V42"
      /></svg
    >
    <h1>Add a gauge.</h1>
    <p>Connect a gauge to PWM{port + 1}, then match its scale.</p>
    <button class="primary full" onclick={oncalibrate} disabled={!status.connected}
      >Add gauge <span>+</span></button
    >
    {#if !status.connected}<p class="hint">Plug your board into USB to begin.</p>{/if}
  </div>
{:else}
  <input
    class="gauge-name"
    aria-label="Gauge name"
    maxlength="64"
    placeholder={source.name}
    value={channel.name}
    oninput={(e) => onchange({ name: e.currentTarget.value })}
    disabled={!status.connected}
  />
  <div class="readout">
    <Dial
      {position}
      min={channel.input_min ?? 0}
      max={source.group === 'Clock' ? source.scale : source.id === 'constant' ? 100 : channel.scale}
      label="Commanded needle position"
    />
    <div class="reading">
      {#if !status.connected || status.paused}<span>—</span>{:else if clock}<span class="clock-reading"
          >{status.board.clock_valid ? clockText : '—'}</span
        >{:else}<span
          >{value == null ? '—' : value.toFixed(value < 10 && source.unit === 'MiB/s' ? 2 : 0)}</span
        ><span class="reading-unit">{source.unit}</span>{/if}
    </div>
    <span class="readout-caption"
      >{status.paused
        ? 'Paused'
        : !status.connected
          ? 'Board disconnected'
          : clock
            ? 'Local time'
            : value == null
              ? 'Waiting for a reading'
              : source.name}</span
    >
  </div>
  <div class="field source-field">
    <label for="source">Source</label>
    <SourcePicker
      value={channel.source}
      options={[...sources, ...status.sources]}
      disabled={!status.connected}
      onchange={(s) => onchange({ source: s.id, scale: s.scale, input_min: s.minimum ?? 0 })}
    />
    <p class="hint">{source.description}</p>
  </div>
  {#if !['Clock', 'Waveforms'].includes(source.group) && source.id !== 'constant' && !['cpu', 'memory', 'swap', 'disk', 'battery', 'esp_rssi'].includes(source.id)}
    <div class="field inline-field">
      <label for="scale">Full scale</label>
      <div class="number-unit">
        <input
          id="scale"
          type="number"
          min="0.1"
          max="1000000000"
          step="1"
          value={channel.scale}
          onchange={(e) => {
            const n = Number(e.currentTarget.value);
            if (n > Math.max(0, channel.input_min ?? 0) && n <= 1e9) onchange({ scale: n });
          }}
          disabled={!status.connected}
        /><span>{source.unit}</span>
      </div>
    </div>
  {:else if source.id === 'constant'}
    <div class="field">
      <div class="value-row">
        <label for="fixed">Position</label><span class="mono">{channel.scale}%</span>
      </div>
      <input
        id="fixed"
        type="range"
        min="0.1"
        max="100"
        step="0.1"
        value={channel.scale}
        oninput={(e) => onchange({ scale: Number(e.currentTarget.value) })}
        disabled={!status.connected}
      />
    </div>
  {/if}
  {#if source.group === 'Waveforms'}
    <div class="field inline-field">
      <label for="period">Period</label>
      <div class="number-unit">
        <input
          id="period"
          type="number"
          min="0.1"
          max="86400"
          step="0.1"
          value={Number(channel.period_s ?? 10)}
          onchange={(e) => {
            const n = Number(e.currentTarget.value);
            if (n >= 0.1 && n <= 86400) onchange({ period_s: n });
          }}
          disabled={!status.connected}
        /><span>s</span>
      </div>
    </div>
    <div class="field">
      <div class="value-row">
        <label for="phase">Phase</label><span class="mono subtle">{Number(channel.phase_deg ?? 0)}°</span>
      </div>
      <input
        id="phase"
        type="range"
        min="0"
        max="360"
        step="5"
        value={Number(channel.phase_deg ?? 0)}
        oninput={(e) => onchange({ phase_deg: Number(e.currentTarget.value) })}
        disabled={!status.connected}
      />
    </div>
  {/if}
  {#if (channel.input_min ?? 0) !== 0 || source.group === 'Super Tracker'}
    <div class="field inline-field">
      <label for="input-min">Scale starts at</label>
      <div class="number-unit">
        <input
          id="input-min"
          type="number"
          max={channel.scale - 0.1}
          step="0.1"
          value={channel.input_min ?? 0}
          onchange={(e) => {
            const n = Number(e.currentTarget.value);
            if (Number.isFinite(n) && n < channel.scale) onchange({ input_min: n });
          }}
          disabled={!status.connected}
        /><span>{source.unit}</span>
      </div>
    </div>
  {/if}
  <div class="field response-field">
    <div class="value-row">
      <label for="response">Needle response</label><span class="mono subtle"
        >{channel.response_ms === 0 ? 'Immediate' : `${(channel.response_ms / 1000).toFixed(1)} s`}</span
      >
    </div>
    <input
      id="response"
      type="range"
      min="0"
      max="2000"
      step="100"
      value={channel.response_ms}
      oninput={(e) => onchange({ response_ms: Number(e.currentTarget.value) })}
      disabled={!status.connected}
    />
    <div class="range-labels"><span>Quick</span><span>Soft</span></div>
  </div>
  <div class="field inline-field">
    <label for="reverse">Reverse direction</label><input
      class="switch"
      id="reverse"
      type="checkbox"
      checked={channel.reverse}
      onchange={(e) => onchange({ reverse: e.currentTarget.checked })}
      disabled={!status.connected}
    />
  </div>
  {#if ['Clock', 'On board', 'Waveforms'].includes(source.group)}<p class="standalone-note">
      <span>↳</span> Runs on the board{clock
        ? '. Add Wi-Fi in board settings to recover time after power loss.'
        : ', even without this app.'}
    </p>{/if}
  {#if source.detail}<p class="hint detail">{source.detail}</p>{/if}
  <div class="inspector-bottom">
    <button class="text-button" onclick={oncalibrate} disabled={!status.connected}
      >Recalibrate <span class="mono subtle"
        >{((channel.min_duty ?? 0) / 10).toFixed(1)}–{(channel.max_duty / 10).toFixed(1)}%</span
      ></button
    ><button
      class="icon-button remove"
      aria-label="Remove gauge"
      title="Remove gauge"
      onclick={onremove}
      disabled={!status.connected}
      ><svg viewBox="0 0 24 24" aria-hidden="true"
        ><path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 10v7m4-7v7" /></svg
      ></button
    >
  </div>
{/if}
