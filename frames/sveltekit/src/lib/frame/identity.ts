function randomId(prefix: string): string {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi && 'randomUUID' in cryptoApi) {
    return `${prefix}_${cryptoApi.randomUUID().replaceAll('-', '')}`;
  }
  return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2)}`;
}

export function stableBrowserId(storageKey: string, prefix: string): string {
  if (typeof localStorage === 'undefined') {
    return randomId(prefix);
  }

  const existing = localStorage.getItem(storageKey);
  if (existing) {
    return existing;
  }

  const created = randomId(prefix);
  localStorage.setItem(storageKey, created);
  return created;
}
