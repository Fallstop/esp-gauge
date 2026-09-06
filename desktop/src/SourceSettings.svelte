<script lang="ts">
  import type { Channel, Source } from './model';
  import ProductTracker from './ProductTracker.svelte';
  let {
    channel,
    source,
    disabled,
    onchange,
  }: { channel: Channel; source: Source; disabled: boolean; onchange: (patch: Partial<Channel>) => void } =
    $props();
  let label = $derived(
    source.id === 'disk'
      ? 'Drive'
      : source.id.startsWith('network_')
        ? 'Network interface'
        : source.id.includes('temperature')
          ? 'Temperature sensor'
          : 'Graphics processor',
  );
  let automatic = $derived(
    source.id === 'disk'
      ? 'System drive'
      : source.id.startsWith('network_')
        ? 'All interfaces'
        : source.id.includes('temperature')
          ? 'Hottest available sensor'
          : 'Busiest available GPU',
  );
</script>

{#if ['disk', 'network_down', 'network_up', 'gpu', 'cpu_temperature', 'gpu_temperature'].includes(source.id)}
  <div class="field source-settings">
    <label for="source-device">{label}</label>
    <select
      id="source-device"
      value={channel.device ?? ''}
      {disabled}
      onchange={(e) => onchange({ device: e.currentTarget.value })}
    >
      <option value="">{automatic}</option>
      {#each source.options ?? [] as option}<option value={option.id}>{option.name}</option>{/each}
      {#if channel.device && !source.options?.some((o) => o.id === channel.device)}<option
          value={channel.device}>{channel.device} · unavailable</option
        >{/if}
    </select>
  </div>
{:else if source.id === 'supertracker_product'}
  <ProductTracker {channel} {disabled} {onchange} />
{/if}
