<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import Dial from './Dial.svelte';
  import type { Channel } from './model';
  let {
    port,
    channel,
    send,
    onfinish,
    oncancel,
  }: {
    port: number;
    channel: Channel;
    send: (op: string, data?: Record<string, unknown>) => Promise<unknown>;
    onfinish: (min: number, max: number) => Promise<boolean>;
    oncancel: () => Promise<void>;
  } = $props();
  let low = $state(untrack(() => channel.min_duty ?? 0));
  let high = $state(untrack(() => channel.max_duty));
  let active = $state<'low' | 'high'>('high');
  let duty = $state(0),
    closing = $state(false),
    error = $state('');
  let running: Promise<unknown> | null = null;
  async function update() {
    if (closing || running) return;
    const sent = duty;
    running = send('calibrate', { port, duty: sent });
    try {
      await running;
      error = '';
    } catch (e) {
      error = String(e);
    } finally {
      running = null;
      if (sent !== duty && !closing) void update();
    }
  }
  function adjust(end: 'low' | 'high', value: number) {
    active = end;
    value = Math.round(Math.max(0, Math.min(1000, value)));
    if (end === 'low') low = Math.min(value, high);
    else high = Math.max(value, low);
    duty = end === 'low' ? low : high;
    void update();
  }
  async function finish() {
    closing = true;
    try {
      await running;
      if (!(await onfinish(low, high))) closing = false;
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
      /* The board also expires a lost preview. */
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
  <div class="eyebrow">PWM{port + 1} · CALIBRATION</div>
  <h1>Match your scale.</h1>
  <p class="intro">Move each end to its mark on the gauge. Start low and watch the needle.</p>
  <div class="calibration-illustration">
    <Dial max={100} position={duty / 1000} label="PWM output, not measured needle position" />
    <span class="live-dot">{(duty / 10).toFixed(1)}% output</span>
  </div>
  <div class="calibration-ends">
    {#each ['low', 'high'] as end}
      <div class:chosen={active === end}>
        <button
          class="text-button"
          onclick={() => adjust(end as 'low' | 'high', end === 'low' ? low : high)}
          disabled={closing}
        >
          {end === 'low' ? 'Zero mark' : 'Full-scale mark'}
        </button>
        <div>
          <input
            class="number-inline"
            type="number"
            aria-label={end === 'low' ? 'Zero mark PWM percent' : 'Full-scale PWM percent'}
            min={end === 'low' ? 0 : low / 10}
            max={end === 'low' ? high / 10 : 100}
            step="0.1"
            value={(end === 'low' ? low : high) / 10}
            disabled={closing}
            oninput={(e) => adjust(end as 'low' | 'high', Number(e.currentTarget.value) * 10)}
          /><span class="unit">%</span>
        </div>
      </div>
    {/each}
  </div>
  <div class="range-slider" style:--low="{low / 10}%" style:--high="{high / 10}%">
    <div class="range-rail"></div>
    <input
      type="range"
      min="0"
      max="1000"
      step="1"
      value={low}
      aria-label="Zero mark output"
      aria-valuemax={high}
      class:range-active={active === 'low'}
      oninput={(e) => adjust('low', Number(e.currentTarget.value))}
      disabled={closing}
    />
    <input
      type="range"
      min="0"
      max="1000"
      step="1"
      value={high}
      aria-label="Full-scale output"
      aria-valuemin={low}
      class:range-active={active === 'high'}
      oninput={(e) => adjust('high', Number(e.currentTarget.value))}
      disabled={closing}
    />
  </div>
  <div class="range-labels"><span>0%</span><span>100%</span></div>
  <p class="hint">Use the zero mark if your scale starts above the needle’s resting position.</p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <button class="primary full" onclick={finish} disabled={high <= low || closing}
    >Use this range <span>↗</span></button
  >
  <button class="text-button cancel" onclick={cancel} disabled={closing}>Cancel calibration</button>
</div>
