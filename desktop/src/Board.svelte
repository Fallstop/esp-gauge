<script lang="ts">
  import type { Config, Snapshot } from './model';
  import { sourceFor } from './model';
  let {
    config,
    status,
    selected,
    onselect,
  }: { config: Config; status: Snapshot; selected: number; onselect: (n: number) => void } = $props();
  const ports = [
    {
      x: 15,
      y: 8,
      tx: 204,
      ty: 315,
      path: '150,107 150,214 204,247 204,315',
      polygon: '145,283 224,242 273,285 273,389 224,417 145,364',
    },
    {
      x: 43,
      y: 8,
      tx: 303,
      ty: 260,
      path: '430,107 430,145 303,221 303,260',
      polygon: '240,231 322,189 369,230 369,330 322,364 240,312',
    },
    {
      x: 71,
      y: 8,
      tx: 400,
      ty: 205,
      path: '710,107 555,107 400,183 400,205',
      polygon: '338,179 416,141 465,178 465,279 420,309 338,261',
    },
    {
      x: 34,
      y: 91,
      tx: 486,
      ty: 559,
      path: '340,632 340,615 486,615 486,559',
      polygon: '425,538 504,481 555,525 555,606 504,641 425,598',
    },
    {
      x: 61,
      y: 91,
      tx: 585,
      ty: 504,
      path: '610,632 610,585 585,572 585,504',
      polygon: '525,483 600,432 650,470 650,556 602,586 525,545',
    },
    {
      x: 88,
      y: 91,
      tx: 686,
      ty: 449,
      path: '880,632 880,541 686,503 686,449',
      polygon: '622,426 700,378 748,419 748,499 702,535 622,491',
    },
  ];
</script>

<div class="board-scene">
  <img class="board-shadow" src="/assets/board-shadow.png" alt="" draggable="false" />
  <img
    class="board-line"
    src="/assets/board-line.png"
    alt="ESP Gauge circuit board. PWM1 to PWM3 on the rear row; PWM4 to PWM6 on the front row."
    draggable="false"
  />
  <svg class="traces" viewBox="0 0 1000 740" aria-hidden="true">
    {#each ports as p, i}
      <polyline points={p.path} class:chosen={selected === i} />
      <circle cx={p.tx} cy={p.ty} r={selected === i ? 6 : 3} class:chosen={selected === i} />
      <polygon points={p.polygon} class:chosen={selected === i} />
    {/each}
  </svg>
  {#each ports as p, i}
    <button
      class="port"
      class:selected={selected === i}
      class:assigned={config.channels[i].enabled}
      style:left="{p.x}%"
      style:top="{p.y}%"
      onclick={() => onselect(i)}
      aria-label="PWM{i + 1}, {config.channels[i].enabled
        ? sourceFor(config.channels[i].source).name
        : 'add gauge'}"
      aria-pressed={selected === i}
    >
      <span class="port-top"
        ><span>PWM{i + 1}</span><span class="port-symbol">{config.channels[i].enabled ? '●' : '+'}</span
        ></span
      >
      <span class="port-source"
        >{config.channels[i].enabled
          ? config.channels[i].name || sourceFor(config.channels[i].source).name
          : 'Add gauge'}</span
      >
      {#if config.channels[i].enabled}<span
          class="port-progress"
          style:width="{Math.max(0, Math.min(100, (status.board.positions?.[i] ?? 0) * 100))}%"
        ></span>{/if}
    </button>
    <button
      class="header-hit"
      style:left="{p.tx / 10}%"
      style:top="{p.ty / 7.4}%"
      onclick={() => onselect(i)}
      aria-label="Select physical header PWM{i + 1}"
      tabindex="-1"
    ></button>
  {/each}
</div>
