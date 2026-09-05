<script lang="ts">
  import { tick } from 'svelte';
  import { sources, sourceFor, type Source } from './model';
  let {
    value,
    options = sources,
    disabled = false,
    onchange,
  }: { value: string; options?: Source[]; disabled?: boolean; onchange: (source: Source) => void } = $props();
  let open = $state(false),
    query = $state(''),
    trigger: HTMLButtonElement;
  let panel = $state<HTMLDivElement>(),
    search = $state<HTMLInputElement>();
  let top = $state(0),
    left = $state(0),
    width = $state(280);
  let matches = $derived(
    options.filter((s) => `${s.name} ${s.group}`.toLowerCase().includes(query.toLowerCase())),
  );
  async function show() {
    const rect = trigger.getBoundingClientRect();
    left = rect.left;
    width = rect.width;
    top = Math.max(12, rect.top - 366);
    query = '';
    open = true;
    await tick();
    search?.focus();
  }
  function choose(source: Source) {
    onchange(source);
    open = false;
    trigger.focus();
  }
  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      open = false;
      trigger.focus();
      return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const options = Array.from(panel?.querySelectorAll<HTMLButtonElement>('[role=option]') ?? []);
      const index = options.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === 'ArrowDown' ? index + 1 : index < 0 ? options.length - 1 : index - 1;
      options[(next + options.length) % options.length]?.focus();
    }
    if (event.key === 'Enter' && document.activeElement === search && matches[0]) {
      event.preventDefault();
      choose(matches[0]);
    }
  }
</script>

<button
  id="source"
  class="source-trigger"
  aria-label="Source"
  aria-haspopup="listbox"
  aria-expanded={open}
  {disabled}
  bind:this={trigger}
  onclick={() => (open ? (open = false) : void show())}
  ><span>{(options.find((s) => s.id === value) ?? sourceFor(value)).name}</span><span aria-hidden="true"
    >⌄</span
  ></button
>
{#if open}
  <button
    class="picker-backdrop"
    tabindex="-1"
    aria-label="Close source picker"
    onclick={() => (open = false)}
  ></button>
  <div
    class="source-picker"
    bind:this={panel}
    style:top="{top}px"
    style:left="{left}px"
    style:width="{width}px"
    onkeydown={keydown}
    role="presentation"
  >
    <input aria-label="Find a source" placeholder="Find a source…" bind:this={search} bind:value={query} />
    <div role="listbox" aria-label="Gauge source" class="source-options">
      {#each [...new Set(matches.map((s) => s.group))] as group}
        {@const entries = matches.filter((s) => s.group === group)}
        {#if entries.length}<div role="group" aria-label={group}>
            <span class="source-group">{group}</span>{#each entries as source}<button
                type="button"
                role="option"
                aria-selected={source.id === value}
                onclick={() => choose(source)}
                ><span>{source.name}</span>{#if source.id === value}<span aria-hidden="true">✓</span
                  >{/if}</button
              >{/each}
          </div>{/if}
      {/each}
      {#if !matches.length}<p class="no-sources">No matching sources.</p>{/if}
    </div>
  </div>
{/if}
