<script lang="ts">
  import type { FramedJsonPayload, MajorMode } from '$lib/mother/types';

  interface Props {
    payload: FramedJsonPayload | null;
    mode?: MajorMode;
  }

  let { payload, mode = 'custom' }: Props = $props();

  type TablePayload = {
    columns?: unknown;
    rows?: unknown;
  };

  function asRecord(value: unknown): Record<string, unknown> | null {
    return typeof value === 'object' && value !== null && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : null;
  }

  function columns(payloadJson: unknown): string[] {
    const record = asRecord(payloadJson) as TablePayload | null;
    if (!Array.isArray(record?.columns)) return [];
    return record.columns.map((column) => String(column));
  }

  function rows(payloadJson: unknown): Record<string, unknown>[] {
    const record = asRecord(payloadJson) as TablePayload | null;
    if (!Array.isArray(record?.rows)) return [];
    return record.rows.map((row) => asRecord(row) ?? { value: row });
  }

  function jsonText(value: unknown): string {
    return JSON.stringify(value, null, 2);
  }

  function stringField(payloadJson: unknown, field: string): string | null {
    const record = asRecord(payloadJson);
    const value = record?.[field];
    return typeof value === 'string' ? value : null;
  }

  let tableColumns = $derived(payload ? columns(payload.json) : []);
  let tableRows = $derived(payload ? rows(payload.json) : []);
  let renderAsTable = $derived(
    Boolean(payload && (mode === 'table' || mode === 'list') && tableColumns.length > 0)
  );
  let markdownContent = $derived(payload ? stringField(payload.json, 'content') : null);
  let markdownPath = $derived(payload ? stringField(payload.json, 'path') : null);
  let markdownGitStatus = $derived(payload ? stringField(payload.json, 'git_status') : null);
</script>

{#if !payload}
  <section class="empty-state">
    <h2>No payload selected</h2>
    <p>Select a Mother-owned buffer and fetch its framed JSON payload.</p>
  </section>
{:else}
  <section class="payload-shell" aria-live="polite">
    <header>
      <div>
        <p class="eyebrow">{payload.frame.protocol}</p>
        <h2>{payload.frame.buffer_id}</h2>
      </div>
      <dl>
        <div><dt>shape</dt><dd>{payload.frame.shape_id}@v{payload.frame.shape_version}</dd></div>
        <div><dt>payload</dt><dd>{payload.frame.payload_contract} v{payload.frame.payload_version}</dd></div>
      </dl>
    </header>

    {#if renderAsTable}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              {#each tableColumns as column}
                <th>{column}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each tableRows as row}
              <tr>
                {#each tableColumns as column}
                  <td>{String(row[column] ?? '')}</td>
                {/each}
              </tr>
            {:else}
              <tr><td colspan={tableColumns.length}>Mother returned no rows.</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if mode === 'markdown' && markdownContent !== null}
      <article class="markdown-render">
        <div class="document-meta">
          {#if markdownPath}<span>{markdownPath}</span>{/if}
          {#if markdownGitStatus}<span>git: {markdownGitStatus}</span>{/if}
        </div>
        <pre>{markdownContent}</pre>
      </article>
    {:else if mode === 'markdown' || mode === 'document' || mode === 'log'}
      <pre class="document-render">{jsonText(payload.json)}</pre>
    {:else}
      <pre class="json-render">{jsonText(payload.json)}</pre>
    {/if}
  </section>
{/if}

<style>
  .empty-state,
  .payload-shell {
    border: 1px solid #243040;
    border-radius: 16px;
    background: #0f1722;
    padding: 1rem;
  }

  .payload-shell header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  h2,
  p {
    margin: 0;
  }

  .eyebrow {
    color: #8fb3ff;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  dl {
    display: grid;
    gap: 0.35rem;
    margin: 0;
    color: #cbd5e1;
    font-size: 0.85rem;
  }

  dt {
    color: #94a3b8;
  }

  dd {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .table-wrap {
    overflow: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
  }

  th,
  td {
    border-bottom: 1px solid #243040;
    padding: 0.55rem;
    text-align: left;
  }

  th {
    color: #bfdbfe;
    background: #111c2b;
  }

  pre {
    overflow: auto;
    white-space: pre-wrap;
    border-radius: 12px;
    background: #070b12;
    color: #e2e8f0;
    padding: 1rem;
  }

  .markdown-render {
    display: grid;
    gap: 0.75rem;
  }

  .document-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    color: #94a3b8;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.8rem;
  }

  .markdown-render pre {
    line-height: 1.55;
  }
</style>
