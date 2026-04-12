<script lang="ts">
  let summary: Record<string, unknown> | null = null;
  let error = '';

  async function load() {
    error = '';
    try {
      const response = await fetch('http://localhost:50051/atlas/atlas.json');
      if (!response.ok) {
        throw new Error(`atlas fetch failed: ${response.status}`);
      }
      const json = await response.json();
      summary = json.summary ?? null;
    } catch (e) {
      error = e instanceof Error ? e.message : 'unknown atlas ui error';
    }
  }

  load();
</script>

<main>
  <h1>Patina Atlas UI (Svelte)</h1>
  {#if error}
    <p style="color: #b91c1c">{error}</p>
  {/if}
  {#if summary}
    <pre>{JSON.stringify(summary, null, 2)}</pre>
  {/if}
</main>
