<script lang="ts">
  import type { Config, Snapshot } from './model';
  import { sourceFor } from './model';
  import { headerTargets, headerViewBox } from './boardTargets';
  let {
    config,
    status,
    selected,
    onselect,
  }: { config: Config; status: Snapshot; selected: number; onselect: (n: number) => void } = $props();
  let hovered = $state<number | null>(null);
  let focused = $state<number | null>(null);
  const ports = [
    {
      x: 15,
      y: 8,
      tx: 204,
      ty: 315,
      path: '150,107 150,214 204,247 204,315',
    },
    {
      x: 43,
      y: 8,
      tx: 303,
      ty: 260,
      path: '430,107 430,120 303,185 303,260',
    },
    {
      x: 71,
      y: 8,
      tx: 400,
      ty: 205,
      path: '710,107 555,107 400,183 400,205',
    },
    {
      x: 34,
      y: 91,
      tx: 486,
      ty: 559,
      path: '340,632 340,615 486,615 486,559',
    },
    {
      x: 61,
      y: 91,
      tx: 585,
      ty: 504,
      path: '610,632 610,585 585,572 585,504',
    },
    {
      x: 88,
      y: 91,
      tx: 686,
      ty: 449,
      path: '880,632 880,541 686,503 686,449',
    },
  ];
</script>

<div class="board-scene" inert={!status.connected}>
  <img class="board-shadow" src="/assets/board-shadow.png" alt="" draggable="false" />
  <img
    class="board-line"
    src="/assets/board-line.png"
    alt="ESP Gauge circuit board. PWM1 to PWM3 on the rear row; PWM4 to PWM6 on the front row."
    draggable="false"
  />
  <svg class="traces" viewBox="0 0 1000 740" aria-hidden="true">
    {#each ports as p, i}
      <polyline pathLength="1" points={p.path} class:chosen={selected === i} />
      <circle cx={p.tx} cy={p.ty} r={selected === i ? 6 : 3} class:chosen={selected === i} />
    {/each}
  </svg>
  <svg class="header-targets" viewBox={headerViewBox} aria-label="PWM headers">
    {#each headerTargets as target, i}
      <path
        class="header-target"
        class:chosen={selected === i}
        class:hovered={hovered === i}
        class:focused={focused === i}
        d={target.path}
        vector-effect="non-scaling-stroke"
        role="button"
        aria-label="Select physical header {target.label}"
        aria-pressed={selected === i}
        tabindex="-1"
        onpointerenter={() => (hovered = i)}
        onpointerleave={() => (hovered = null)}
        onfocus={() => (focused = i)}
        onblur={() => (focused = null)}
        onclick={() => onselect(i)}
        onkeydown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            onselect(i);
          }
        }}
      />
    {/each}
  </svg>
  {#each ports as p, i}
    <button
      class="port"
      class:selected={selected === i}
      class:hovered={hovered === i}
      class:assigned={config.channels[i].enabled}
      style:--port-x="{p.x}%"
      style:--port-y="{p.y}%"
      style:--anchor-x="{p.tx / 10}%"
      style:--anchor-y="{p.ty / 7.4}%"
      onclick={() => onselect(i)}
      onpointerenter={() => (hovered = i)}
      onpointerleave={() => (hovered = null)}
      onfocus={() => (focused = i)}
      onblur={() => (focused = null)}
      aria-label="PWM{i + 1}, {config.channels[i].enabled
        ? sourceFor(config.channels[i].source, status).name
        : 'add gauge'}"
      aria-pressed={selected === i}
    >
      <span class="port-top"
        ><span>PWM{i + 1}</span><span class="port-symbol">{config.channels[i].enabled ? '●' : '+'}</span
        ></span
      >
      <span class="port-source"
        >{config.channels[i].enabled
          ? config.channels[i].name || sourceFor(config.channels[i].source, status).name
          : 'Add gauge'}</span
      >
      {#if config.channels[i].enabled}<span
          class="port-progress"
          style:width="{Math.max(0, Math.min(100, (status.board.positions?.[i] ?? 0) * 100))}%"
        ></span>{/if}
    </button>
  {/each}
</div>
