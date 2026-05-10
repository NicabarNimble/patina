<script lang="ts">
  import { onMount } from 'svelte';
  import BufferPayload from '$lib/components/BufferPayload.svelte';
  import { stableBrowserId } from '$lib/frame/identity';
  import {
    MotherApiError,
    connectWindow,
    disconnectWindow,
    getBufferPayload,
    listBuffers,
    listGaps,
    listRequestDetails,
    listShapes,
    listWindows,
    openBuffer,
    openRequestShape
  } from '$lib/mother/client';
  import type {
    FramedJsonPayload,
    ObservabilityGap,
    OpenedBuffer,
    ViewBuffer,
    ViewRequestAction,
    ViewRequestDetail,
    ViewShape,
    ViewWindow
  } from '$lib/mother/types';

  let frameId = 'frame_pending';
  let windowId = 'window_pending';
  let buffers: ViewBuffer[] = [];
  let shapes: ViewShape[] = [];
  let windows: ViewWindow[] = [];
  let gaps: ObservabilityGap[] = [];
  let requestDetails: ViewRequestDetail[] = [];
  let selectedBufferId = '';
  let selectedShapeId = '';
  let payload: FramedJsonPayload | null = null;
  let activeBuffer: ViewBuffer | null = null;
  let loading = false;
  let error = '';
  let notice = '';

  $: selectedBuffer = buffers.find((buffer) => buffer.buffer_id === selectedBufferId) ?? null;
  $: selectedShape = shapes.find((shape) => shape.shape_id === selectedShapeId) ?? null;
  $: connectedWindow = windows.find(
    (window) => window.window_id === windowId && window.connection_state === 'connected'
  );
  $: connectableBuffers = buffers.filter((buffer) =>
    ['live', 'stale', 'blocked'].includes(buffer.state)
  );
  $: activeShapes = shapes.filter((shape) => shape.active);

  function describeError(caught: unknown): string {
    if (caught instanceof MotherApiError) {
      return `${caught.message} (${caught.status})`;
    }
    return caught instanceof Error ? caught.message : String(caught);
  }

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      const [nextBuffers, nextShapes, nextWindows, nextGaps, nextDetails] = await Promise.all([
        listBuffers(),
        listShapes(),
        listWindows(),
        listGaps(),
        listRequestDetails()
      ]);
      buffers = nextBuffers;
      shapes = nextShapes;
      windows = nextWindows;
      gaps = nextGaps;
      requestDetails = nextDetails;
      const nextConnectable = nextBuffers.filter((buffer) =>
        ['live', 'stale', 'blocked'].includes(buffer.state)
      );
      const nextActiveShapes = nextShapes.filter((shape) => shape.active);
      if (!selectedBufferId && nextConnectable.length > 0) {
        selectedBufferId = nextConnectable[0].buffer_id;
      }
      if (!selectedShapeId && nextActiveShapes.length > 0) {
        selectedShapeId = nextActiveShapes[0].shape_id;
      }
    } catch (caught) {
      error = describeError(caught);
    } finally {
      loading = false;
    }
  }

  async function fetchPayload(bufferId = selectedBufferId): Promise<void> {
    if (!bufferId) return;
    error = '';
    notice = '';
    try {
      const opened = await getBufferPayload(bufferId);
      payload = opened.payload;
      activeBuffer = opened.buffer;
      selectedBufferId = opened.buffer.buffer_id;
      notice = `Rendered Mother payload for ${opened.buffer.name}.`;
    } catch (caught) {
      error = describeError(caught);
      payload = null;
      activeBuffer = null;
    }
  }

  async function connectSelected(): Promise<void> {
    if (!selectedBuffer) return;
    error = '';
    notice = '';
    try {
      await connectWindow({
        frame_id: frameId,
        frame_kind: 'sveltekit',
        window_id: windowId,
        buffer_id: selectedBuffer.buffer_id
      });
      await fetchPayload(selectedBuffer.buffer_id);
      await refresh();
      notice = `Connected ${windowId} to ${selectedBuffer.name}.`;
    } catch (caught) {
      error = describeError(caught);
    }
  }

  async function disconnectSelected(): Promise<void> {
    error = '';
    notice = '';
    try {
      await disconnectWindow({ window_id: windowId });
      await refresh();
      notice = `Disconnected ${windowId}; Mother buffer remains alive.`;
    } catch (caught) {
      error = describeError(caught);
    }
  }

  function applyOpened(opened: OpenedBuffer): void {
    payload = opened.payload;
    activeBuffer = opened.buffer;
    selectedBufferId = opened.buffer.buffer_id;
  }

  async function openSelectedShape(): Promise<void> {
    if (!selectedShape) return;
    error = '';
    notice = '';
    try {
      const opened = await openBuffer({ shape_id: selectedShape.shape_id });
      applyOpened(opened);
      await refresh();
      notice = `Mother opened ${opened.buffer.name}.`;
    } catch (caught) {
      error = describeError(caught);
    }
  }

  function openedFromRequestOutcome(outcome: unknown): OpenedBuffer | null {
    if (typeof outcome !== 'object' || outcome === null) return null;
    const record = outcome as Record<string, unknown>;
    if (record.outcome !== 'opened') return null;
    if (typeof record.buffer !== 'object' || record.buffer === null) return null;
    if (typeof record.payload !== 'object' || record.payload === null) return null;
    return record as unknown as OpenedBuffer;
  }

  async function openAction(detail: ViewRequestDetail, action: ViewRequestAction): Promise<void> {
    error = '';
    notice = '';
    try {
      const outcome = await openRequestShape({
        request_id: detail.request.request_id,
        shape_id: action.shape_id
      });
      const opened = openedFromRequestOutcome(outcome.open_outcome);
      if (opened) {
        applyOpened(opened);
        notice = `${action.label}: Mother opened ${opened.buffer.name}.`;
      } else {
        notice = `${action.label}: Mother returned a non-opened outcome.`;
      }
      await refresh();
    } catch (caught) {
      error = describeError(caught);
    }
  }

  onMount(() => {
    frameId = stableBrowserId('patina.mother.sveltekit.frame_id', 'frame_sveltekit');
    windowId = stableBrowserId('patina.mother.sveltekit.window_id', 'window_sveltekit');
    void refresh();
  });
</script>

<svelte:head>
  <title>Patina Mother Frame</title>
  <meta
    name="description"
    content="SvelteKit renderer frame for Mother-owned Patina view buffers."
  />
</svelte:head>

<main>
  <header class="hero">
    <div>
      <p class="eyebrow">Mother View Composer</p>
      <h1>SvelteKit Frame</h1>
      <p class="summary">
        Renderer-only client for Mother-owned buffers, shapes, requests, windows, and gaps.
      </p>
    </div>
    <button on:click={refresh} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh Mother'}</button>
  </header>

  {#if error}
    <aside class="banner error">{error}</aside>
  {/if}
  {#if notice}
    <aside class="banner notice">{notice}</aside>
  {/if}

  <section class="identity-card">
    <div><strong>frame</strong><code>{frameId}</code></div>
    <div><strong>window</strong><code>{windowId}</code></div>
    <div><strong>connected buffer</strong><code>{connectedWindow?.buffer_id ?? 'none'}</code></div>
  </section>

  <div class="layout">
    <aside class="panel">
      <h2>Buffers</h2>
      {#if connectableBuffers.length === 0}
        <p class="muted">No live, stale, or blocked Mother buffers.</p>
      {:else}
        <label>
          Mother buffer
          <select bind:value={selectedBufferId}>
            {#each connectableBuffers as buffer}
              <option value={buffer.buffer_id}>{buffer.name} · {buffer.state}</option>
            {/each}
          </select>
        </label>
        <div class="button-row">
          <button on:click={() => fetchPayload()} disabled={!selectedBuffer}>Render payload</button>
          <button on:click={connectSelected} disabled={!selectedBuffer}>Connect</button>
          <button on:click={disconnectSelected} disabled={!connectedWindow}>Disconnect</button>
        </div>
      {/if}

      <h2>Open shape</h2>
      {#if activeShapes.length === 0}
        <p class="muted">No active shapes available.</p>
      {:else}
        <label>
          Active shape
          <select bind:value={selectedShapeId}>
            {#each activeShapes as shape}
              <option value={shape.shape_id}>{shape.title} · {shape.maturity}</option>
            {/each}
          </select>
        </label>
        <button on:click={openSelectedShape} disabled={!selectedShape}>Ask Mother to open shape</button>
      {/if}

      <h2>Request actions</h2>
      {#each requestDetails as detail}
        {#if detail.available_actions.length > 0}
          <article class="request-card">
            <p>{detail.request.raw_request}</p>
            {#each detail.available_actions as action}
              <button on:click={() => openAction(detail, action)}>{action.label}</button>
            {/each}
          </article>
        {/if}
      {:else}
        <p class="muted">No request-linked open actions.</p>
      {/each}
    </aside>

    <section class="workspace">
      <BufferPayload payload={payload} mode={activeBuffer?.major_mode ?? selectedBuffer?.major_mode ?? 'custom'} />

      <section class="panel inline-grid">
        <article>
          <h3>Windows</h3>
          <ul>
            {#each windows as window}
              <li><code>{window.window_id}</code> {window.connection_state} → {window.buffer_id ?? 'none'}</li>
            {:else}
              <li class="muted">No Mother window records.</li>
            {/each}
          </ul>
        </article>
        <article>
          <h3>Observability gaps</h3>
          <ul>
            {#each gaps as gap}
              <li><code>{gap.missing_fact_path}</code> · {gap.status}</li>
            {:else}
              <li class="muted">No view observability gaps.</li>
            {/each}
          </ul>
        </article>
      </section>
    </section>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    background: #070b12;
    color: #e5edf6;
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  }

  main {
    max-width: 1440px;
    margin: 0 auto;
    padding: 2rem;
  }

  .hero,
  .identity-card,
  .layout,
  .button-row,
  .inline-grid {
    display: grid;
    gap: 1rem;
  }

  .hero {
    grid-template-columns: 1fr auto;
    align-items: center;
    margin-bottom: 1rem;
  }

  .layout {
    grid-template-columns: minmax(280px, 360px) 1fr;
    align-items: start;
  }

  .workspace {
    display: grid;
    gap: 1rem;
  }

  .identity-card,
  .panel {
    border: 1px solid #243040;
    border-radius: 16px;
    background: #0f1722;
    padding: 1rem;
  }

  .identity-card {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin-bottom: 1rem;
  }

  .identity-card div,
  label,
  .panel {
    display: grid;
    gap: 0.45rem;
  }

  .inline-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  h1,
  h2,
  h3,
  p {
    margin: 0;
  }

  h2 {
    margin-top: 1rem;
  }

  h2:first-child {
    margin-top: 0;
  }

  .eyebrow {
    color: #8fb3ff;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-size: 0.8rem;
  }

  .summary,
  .muted {
    color: #9fb0c3;
  }

  button,
  select {
    border: 1px solid #334155;
    border-radius: 10px;
    background: #111c2b;
    color: #e5edf6;
    padding: 0.65rem 0.85rem;
  }

  button {
    cursor: pointer;
    background: #1e40af;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  code {
    color: #bfdbfe;
    overflow-wrap: anywhere;
  }

  .banner {
    border-radius: 12px;
    margin-bottom: 1rem;
    padding: 0.85rem 1rem;
  }

  .error {
    background: #3f121b;
    border: 1px solid #7f1d1d;
  }

  .notice {
    background: #0f2e1c;
    border: 1px solid #166534;
  }

  .request-card {
    display: grid;
    gap: 0.5rem;
    border-top: 1px solid #243040;
    padding-top: 0.75rem;
  }

  ul {
    margin: 0;
    padding-left: 1rem;
  }

  @media (max-width: 900px) {
    main {
      padding: 1rem;
    }

    .hero,
    .layout,
    .identity-card,
    .inline-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
