<script lang="ts">
  import { tick } from 'svelte';
  import { sources, sourceFor, type Source } from './model';
  let {
    value,
    options = sources,
    disabled = false,
    onchange,
  }: {
    value: string;
    options?: Source[];
    disabled?: boolean;
    onchange: (source: Source) => void;
  } = $props();
  const categories = [
    { id: 'computer', name: 'This computer', detail: 'Processor, graphics, storage & sound' },
    { id: 'board', name: 'ESP32 board', detail: 'Sensors, clocks & waveforms' },
    { id: 'ai', name: 'AI tools', detail: 'Agents, activity & usage limits' },
    { id: 'prices', name: 'Prices', detail: 'Products & NZ food prices' },
  ];
  const categoryFor = (s: Source) =>
    ['This computer', 'Computer'].includes(s.group)
      ? 'computer'
      : ['ESP32 board', 'On board', 'Clock', 'Waveforms'].includes(s.group)
        ? 'board'
        : s.group === 'Super Tracker'
          ? 'prices'
          : 'ai';
  let open = $state(false),
    query = $state(''),
    category = $state('');
  let trigger: HTMLButtonElement,
    panel = $state<HTMLDivElement>(),
    search = $state<HTMLInputElement>();
  let top = $state(0),
    left = $state(0),
    width = $state(360),
    height = $state(460);
  let matches = $derived(
    options.filter((s) =>
      query.trim()
        ? `${s.name} ${s.group} ${categories.find((c) => c.id === categoryFor(s))?.name}`
            .toLowerCase()
            .includes(query.trim().toLowerCase())
        : categoryFor(s) === category,
    ),
  );
  let current = $derived(options.find((s) => s.id === value) ?? sourceFor(value));
  async function show() {
    const rect = trigger.getBoundingClientRect();
    width = Math.min(380, window.innerWidth - 24);
    height = Math.min(470, window.innerHeight - 24);
    left = Math.max(12, Math.min(rect.right - width, window.innerWidth - width - 12));
    top = Math.max(12, Math.min(rect.top - height / 2, window.innerHeight - height - 12));
    query = '';
    category = '';
    open = true;
    await tick();
    search?.focus();
  }
  function close() {
    open = false;
    trigger.focus();
  }
  function choose(source: Source) {
    onchange(source);
    close();
  }
  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const entries = Array.from(panel?.querySelectorAll<HTMLButtonElement>('[data-choice]') ?? []).filter(
        (button) => !button.disabled,
      );
      const index = entries.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === 'ArrowDown' ? index + 1 : index < 0 ? entries.length - 1 : index - 1;
      entries[(next + entries.length) % entries.length]?.focus();
    }
    if (event.key === 'Tab' && panel) {
      const entries = Array.from(panel.querySelectorAll<HTMLElement>('button, input'));
      if (event.shiftKey && document.activeElement === entries[0]) {
        event.preventDefault();
        entries.at(-1)?.focus();
      } else if (!event.shiftKey && document.activeElement === entries.at(-1)) {
        event.preventDefault();
        entries[0]?.focus();
      }
    }
    if (event.key === 'Enter' && document.activeElement === search && matches[0] && query.trim()) {
      event.preventDefault();
      choose(matches[0]);
    }
  }
</script>

<svelte:window onresize={() => (open = false)} />
<button
  id="source"
  class="source-trigger"
  aria-label="Source"
  aria-haspopup="dialog"
  aria-expanded={open}
  {disabled}
  bind:this={trigger}
  onclick={() => (open ? close() : void show())}
>
  <span><small>{current.group}</small>{current.name}</span><span aria-hidden="true">⌄</span>
</button>
{#if open}
  <button class="picker-backdrop" tabindex="-1" aria-label="Close source picker" onclick={close}></button>
  <div
    class="source-picker"
    bind:this={panel}
    style:top="{top}px"
    style:left="{left}px"
    style:width="{width}px"
    style:height="{height}px"
    onkeydown={keydown}
    role="dialog"
    aria-modal="true"
    aria-label="Choose a source"
    tabindex="-1"
  >
    <div class="picker-title">
      <strong>Choose a source</strong><button aria-label="Close" onclick={close}>×</button>
    </div>
    <input
      aria-label="Find a source"
      placeholder="Search all sources…"
      bind:this={search}
      bind:value={query}
    />
    {#if !query.trim() && !category}
      <div class="source-categories">
        {#each categories as c}
          {@const count = options.filter((s) => categoryFor(s) === c.id).length}
          <button
            data-choice
            onclick={async () => {
              category = c.id;
              await tick();
              search?.focus();
            }}
            disabled={!count}
          >
            <span><strong>{c.name}</strong><small>{c.detail}</small></span><span aria-hidden="true">→</span>
          </button>
        {/each}
      </div>
      <p class="picker-foot">Currently: {current.group} · {current.name}</p>
    {:else}
      <button
        class="picker-back"
        onclick={() => {
          query = '';
          category = '';
          search?.focus();
        }}
        >← All categories{category && !query.trim()
          ? ` / ${categories.find((c) => c.id === category)?.name}`
          : ''}</button
      >
      <div role="listbox" aria-label="Gauge source" class="source-options">
        {#each [...new Set(matches.map((s) => s.group))] as group}
          <div role="group" aria-label={group}>
            <span class="source-group">{group}</span>
            {#each matches.filter((s) => s.group === group) as source}
              <button
                data-choice
                type="button"
                role="option"
                aria-selected={source.id === value}
                onclick={() => choose(source)}
              >
                <span>{source.name}</span>{#if source.id === value}<span aria-hidden="true">✓</span>{/if}
              </button>
            {/each}
          </div>
        {/each}
        {#if !matches.length}<p class="no-sources">No matching sources. Try a shorter search.</p>{/if}
      </div>
    {/if}
  </div>
{/if}
