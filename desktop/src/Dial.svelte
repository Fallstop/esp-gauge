<script lang="ts">
  let {
    position = 0,
    label = 'Needle position',
    large = false,
    max = 100,
    min = 0,
  }: { position?: number; label?: string; large?: boolean; max?: number; min?: number } = $props();
  const ticks = Array.from({ length: 21 }, (_, i) => i);
  function point(angle: number, r: number) {
    const a = (angle * Math.PI) / 180;
    return { x: 150 + Math.cos(a) * r, y: 130 + Math.sin(a) * r };
  }
  let angle = $derived(200 + Math.max(0, Math.min(1, position)) * 140);
</script>

<svg class="dial" class:large viewBox="0 0 300 170" role="img" aria-label={label}>
  <path d="M37.24 88.96 A120 120 0 0 1 262.76 88.96" class="dial-arc" />
  {#each ticks as i}
    {@const a = 200 + i * 7}{@const p = point(a, 114)}{@const q = point(a, i % 5 === 0 ? 101 : 107)}
    <line x1={p.x} y1={p.y} x2={q.x} y2={q.y} class:major={i % 5 === 0} />
  {/each}
  <text x="29" y="109">{min}</text><text x="150" y="43" text-anchor="middle">{(min + max) / 2}</text><text
    x="264"
    y="109"
    text-anchor="middle">{max}</text
  >
  <g class="needle" style:transform="rotate({angle - 270}deg)" style:transform-origin="150px 130px">
    <path d="M147.5 129 150 50 152.5 129Z" /><line x1="150" y1="130" x2="150" y2="142" />
  </g>
  <circle cx="150" cy="130" r="6" class="hub" /><circle cx="150" cy="130" r="2" class="hub-centre" />
</svg>
