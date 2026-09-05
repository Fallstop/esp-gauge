<script lang="ts">
  import { onMount } from 'svelte';
  import Dial from './Dial.svelte';
  let {
    port,
    send,
    onfinish,
    oncancel,
  }: {
    port: number;
    send: (op: string, data?: Record<string, unknown>) => Promise<unknown>;
    onfinish: (duty: number) => Promise<boolean>;
    oncancel: () => Promise<void>;
  } = $props();
  let duty = $state(0),
    limit = $state(200),
    closing = $state(false),
    error = $state('');
  let running: Promise<unknown> | null = null;
  async function update() {
    if (closing || running) return;
    running = send('calibrate', { port, duty });
    try {
      await running;
      error = '';
    } catch (e) {
      error = String(e);
    } finally {
      running = null;
    }
  }
  async function finish() {
    closing = true;
    try {
      await running;
      if (!(await onfinish(duty))) closing = false;
    } catch (e) {
      error = String(e);
      closing = false;
    }
  }
  async function cancel() {
    closing = true;
    try {
      await running;
    } catch {
      /* The device watchdog also ends a lost preview. */
    }
    await oncancel();
  }
  onMount(() => {
    const timer = setInterval(update, 180);
    return () => {
      closing = true;
      clearInterval(timer);
    };
  });
</script>

<div class="calibration">
  <div class="eyebrow">CALIBRATE · PWM{port + 1}</div>
  <h1>Find full scale.</h1>
  <p class="intro">Raise the output slowly. Stop when your gauge reaches its 100% mark.</p>
  <div class="calibration-illustration">
    <Dial max={88} position={duty / 880} label="PWM output, not measured needle position" /><span
      class="live-dot">Live output</span
    >
  </div>
  <div class="value-row">
    <label for="cal-duty">PWM duty</label>
    <div>
      <input
        id="cal-duty"
        class="number-inline"
        type="number"
        min="0"
        max={limit / 10}
        step="0.1"
        value={duty / 10}
        oninput={(e) => {
          duty = Math.round(Math.max(0, Math.min(limit / 10, Number(e.currentTarget.value))) * 10);
          void update();
        }}
        disabled={closing}
      /><span class="unit">%</span>
    </div>
  </div>
  <input
    class="cal-slider"
    aria-label="Live calibration output"
    type="range"
    min="0"
    max={limit}
    step="1"
    value={duty}
    oninput={(e) => {
      duty = Number(e.currentTarget.value);
      void update();
    }}
    disabled={closing}
  />
  <div class="range-labels">
    <span>0%</span><button
      class="text-button"
      onclick={() => (limit = limit === 200 ? 880 : 200)}
      disabled={closing || duty > 200}>{limit === 200 ? 'Extend range to 88%' : 'Limit range to 20%'}</button
    ><span>{limit / 10}%</span>
  </div>
  <p class="hint">
    Watch the physical needle. The preview shows electrical output, not your gauge’s reading.
  </p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <button class="primary full" onclick={finish} disabled={duty === 0 || closing}
    >This is 100% <span>↗</span></button
  >
  <button class="text-button cancel" onclick={cancel} disabled={closing}>Cancel calibration</button>
</div>
