import { TauriService, SandboxLogPayload } from './tauri';

/**
 * The live console.txt lines, kept outside the Monitor Center so they survive leaving it.
 *
 * They used to be state inside the Monitor, which React throws away when another tab is shown:
 * open the Merger, come back, and a session's fifteen hundred lines were gone with nothing
 * closed. The Rust watcher only streams what is written after it started, so nothing could
 * bring them back. One subscription for the app's lifetime, and the Monitor reads from here.
 */
const MAX_LINES = 1500;

const lines: string[] = [];
const listeners = new Set<() => void>();
let started = false;

function notify() {
  for (const l of listeners) l();
}

export function startLiveConsole() {
  if (started) return;
  started = true;
  void TauriService.listenSandboxLogs((payload: SandboxLogPayload) => {
    lines.push(payload.line);
    if (lines.length > MAX_LINES) lines.splice(0, lines.length - MAX_LINES);
    notify();
  });
}

/** A copy of the lines, oldest first. */
export function liveConsoleLines(): string[] {
  return [...lines];
}

/** Empties the console on screen, as the Monitor's Clear does; the file on disk is untouched. */
export function clearLiveConsole() {
  lines.length = 0;
  notify();
}

export function subscribeLiveConsole(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
