<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { Channel } from './model';
  let {
    channel,
    disabled,
    onchange,
  }: { channel: Channel; disabled: boolean; onchange: (patch: Partial<Channel>) => void } = $props();
  let query = $state(''),
    results = $state<{ id: number; name: string }[]>([]),
    stores = $state<{ id: number; name: string }[]>([]);
  let searching = $state(false),
    loading = $state(false),
    searched = $state(false),
    error = $state(''),
    storeError = $state('');
  let request = 0;
  $effect(() => {
    const product = channel.product_id;
    let cancelled = false;
    stores = [];
    storeError = '';
    loading = !!product;
    if (product)
      void invoke<typeof stores>('product_stores', { product })
        .then((value) => {
          if (!cancelled) stores = value;
        })
        .catch((e) => {
          if (!cancelled) storeError = String(e);
        })
        .finally(() => {
          if (!cancelled) loading = false;
        });
    return () => {
      cancelled = true;
    };
  });
  async function search() {
    const id = ++request;
    searching = true;
    error = '';
    results = [];
    searched = false;
    try {
      const value = await invoke<typeof results>('search_products', { query });
      if (id === request) {
        results = value;
        searched = true;
      }
    } catch (e) {
      if (id === request) error = String(e);
    } finally {
      if (id === request) searching = false;
    }
  }
</script>

<div class="field source-settings product-tracker">
  <label for="product-query">Product</label>
  {#if channel.product_id}<p class="selected-product">
      {channel.product_name || `Product ${channel.product_id}`}
    </p>{/if}
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void search();
    }}
  >
    <input id="product-query" placeholder="Name or barcode…" bind:value={query} maxlength="120" {disabled} />
    <button class="search-product" disabled={disabled || searching || query.trim().length < 2}
      >{searching ? 'Searching…' : 'Find'}</button
    >
  </form>
  {#if error}<p class="hint error" role="alert">{error}</p>{/if}
  {#if searched && !results.length}<p class="hint">No products found. Try a brand or barcode.</p>{/if}
  {#if results.length}<div class="product-results" aria-label="Matching products">
      {#each results as product}<button
          {disabled}
          onclick={() => {
            onchange({
              product_id: product.id,
              product_name: product.name.slice(0, 120),
              store_id: 0,
              store_name: '',
            });
            results = [];
            searched = false;
            query = '';
          }}>{product.name}</button
        >{/each}
    </div>{/if}
  {#if channel.product_id}
    <label for="product-store">Store</label>
    <select
      id="product-store"
      value={channel.store_id ?? 0}
      disabled={disabled || loading}
      onchange={(e) => {
        const id = Number(e.currentTarget.value);
        onchange({ store_id: id, store_name: stores.find((s) => s.id === id)?.name.slice(0, 120) ?? '' });
      }}
    >
      <option value={0}>Lowest national shelf price</option>
      {#each stores as store}<option value={store.id}>{store.name}</option>{/each}
      {#if channel.store_id && !stores.some((s) => s.id === channel.store_id)}<option value={channel.store_id}
          >{channel.store_name || `Store ${channel.store_id}`} · unavailable</option
        >{/if}
    </select>
    <p class="hint">
      {loading
        ? 'Finding stores…'
        : storeError ||
          (channel.store_id
            ? 'Locked to this store. An unavailable price rests the gauge; it never switches stores.'
            : 'Lowest current shelf price across retailers’ national medians. Excludes club and multibuy prices.')}
    </p>
    <p class="hint">Refreshes every five minutes. Prices in NZ dollars.</p>
  {/if}
</div>
