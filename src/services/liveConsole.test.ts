import { beforeEach, describe, expect, it, vi } from 'vitest';

type Listener = (payload: { line: string; is_error: boolean }) => void;
let listener: Listener | null = null;

vi.mock('./tauri', () => ({
  TauriService: {
    listenSandboxLogs: async (cb: Listener) => {
      listener = cb;
      return () => undefined;
    },
  },
}));

describe('liveConsole', () => {
  beforeEach(() => {
    vi.resetModules();
    listener = null;
  });

  it('keeps what the watcher streamed, tells subscribers, and starts only once', async () => {
    const store = await import('./liveConsole');
    const seen: number[] = [];
    store.subscribeLiveConsole(() => seen.push(store.liveConsoleLines().length));
    store.startLiveConsole();
    store.startLiveConsole();
    expect(listener).not.toBeNull();
    listener!({ line: 'one', is_error: false });
    listener!({ line: 'two', is_error: true });
    expect(store.liveConsoleLines()).toEqual(['one', 'two']);
    expect(seen).toEqual([1, 2]);
  });

  it('hands out a copy, so a reader cannot change the store', async () => {
    const store = await import('./liveConsole');
    store.startLiveConsole();
    listener!({ line: 'kept', is_error: false });
    store.liveConsoleLines().push('stray');
    expect(store.liveConsoleLines()).toEqual(['kept']);
  });

  it('forgets the oldest lines past its limit', async () => {
    const store = await import('./liveConsole');
    store.startLiveConsole();
    for (let i = 0; i < 1600; i += 1) listener!({ line: `l${i}`, is_error: false });
    const lines = store.liveConsoleLines();
    expect(lines).toHaveLength(1500);
    expect(lines[0]).toBe('l100');
  });
});
