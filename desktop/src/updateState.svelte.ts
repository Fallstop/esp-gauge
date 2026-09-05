import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
export type UpdateStatus = {
  checked: boolean;
  app_version: string;
  firmware_version: string;
  busy: boolean;
  stage: string;
  progress: number;
  error: string | null;
};
export const updates = $state<UpdateStatus>({
  checked: false,
  app_version: '',
  firmware_version: '',
  busy: false,
  stage: '',
  progress: 0,
  error: null,
});
export function newerVersion(candidate: string, installed: string): boolean {
  const parts = (v: string) =>
    v
      .match(/^(\d+)\.(\d+)\.(\d+)/)
      ?.slice(1)
      .map(Number);
  const next = parts(candidate),
    current = parts(installed);
  if (!next || !current) return false;
  for (let i = 0; i < 3; i++) {
    if (next[i] !== current[i]) return next[i] > current[i];
  }
  return installed.includes('-') && !candidate.includes('-');
}
export async function runUpdate(command: string, args?: Record<string, unknown>) {
  try {
    await invoke(command, args);
  } catch (e) {
    updates.error = String(e);
  }
}
export function startUpdates() {
  if (!isTauri()) return () => {};
  let disposed = false;
  let stop = () => {};
  void listen<UpdateStatus>('updates', (e) => Object.assign(updates, e.payload)).then((fn) => {
    if (disposed) fn();
    else stop = fn;
  });
  void invoke<UpdateStatus>('update_status').then((value) => Object.assign(updates, value));
  const first = setTimeout(() => void runUpdate('check_updates'), 8000);
  const interval = setInterval(
    () => {
      if (!updates.busy) void runUpdate('check_updates');
    },
    6 * 60 * 60 * 1000,
  );
  return () => {
    disposed = true;
    stop();
    clearTimeout(first);
    clearInterval(interval);
  };
}
