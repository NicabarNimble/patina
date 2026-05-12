// See https://svelte.dev/docs/kit/types#app.d.ts
import type { FrameLocals } from '$lib/mother/types';

declare global {
  namespace App {
    interface Locals extends FrameLocals {}
  }
}

export {};
