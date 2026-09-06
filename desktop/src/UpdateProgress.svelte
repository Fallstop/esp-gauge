<script lang="ts">
  import { onMount } from 'svelte';
  import { updates } from './updateState.svelte';
  let seconds = $state(0),
    started = Date.now();
  $effect(() => {
    if (updates.busy) {
      started = Date.now();
      seconds = 0;
    }
  });
  onMount(() => {
    const timer = setInterval(() => {
      if (updates.busy) seconds = Math.floor((Date.now() - started) / 1000);
    }, 1000);
    return () => clearInterval(timer);
  });
</script>

{#if updates.busy || updates.stage || updates.error}
  <div class="board-update" class:update-failed={!!updates.error} role={updates.error ? 'alert' : 'status'}>
    <div class="update-heading">
      <strong
        >{updates.operation === 'firmware'
          ? 'Board firmware'
          : updates.operation === 'app'
            ? 'ESP Gauge update'
            : 'Updates'}</strong
      >
      {#if updates.busy}<span class="mono"
          >{updates.indeterminate ? `${seconds}s elapsed` : `${Math.floor(updates.progress)}%`}</span
        >
      {:else}<button
          aria-label="Dismiss update status"
          onclick={() => {
            updates.stage = '';
            updates.error = null;
          }}>×</button
        >{/if}
    </div>
    <p>{updates.error || updates.stage}</p>
    {#if updates.busy}
      {#if updates.indeterminate}<progress aria-label={updates.stage} max="100"></progress>{:else}<progress
          aria-label={updates.stage}
          max="100"
          value={updates.progress}
        ></progress>{/if}
      <small
        >{updates.operation === 'firmware'
          ? 'Keep USB connected. The board may briefly disconnect while it restarts.'
          : updates.indeterminate
            ? 'Working… this step can take a moment.'
            : `${seconds}s elapsed`}</small
      >
    {/if}
  </div>
{/if}
