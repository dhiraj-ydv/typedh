import type { LinuxTargetInfo } from './native-api';

/** A stock-updater install waiting for user consent. Implemented by the view. */
export interface PendingStockUpdate {
  version: string;
  notes: string;
  install(onProgress: (downloaded: number, total: number | undefined) => void): Promise<void>;
}

export interface UpdateFlowDeps {
  getAppVersion(): Promise<string>;
  /** Resolves null when no newer signed release exists. */
  checkPlugin(): Promise<PendingStockUpdate | null>;
  linuxInfo(): Promise<LinuxTargetInfo>;
}

export type UpdateCheck =
  | { kind: 'up-to-date'; currentVersion: string }
  | {
      kind: 'available-stock';
      currentVersion: string;
      version: string;
      notes: string;
      install: PendingStockUpdate['install'];
    }
  | { kind: 'available-linux'; currentVersion: string; version: string; notes: string }
  | { kind: 'unsupported'; currentVersion: string; reason: string }
  | { kind: 'error'; currentVersion: string | null; message: string };

export function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/**
 * Check for updates. Version discovery always goes through the signed
 * updater manifest; only the install step differs: user-owned Linux
 * `~/.local` installs self-update through the backend, everything else
 * uses the stock Tauri installer flow.
 */
export async function checkForUpdates(deps: UpdateFlowDeps): Promise<UpdateCheck> {
  let currentVersion: string | null = null;
  try {
    currentVersion = await deps.getAppVersion();
    const pending = await deps.checkPlugin();
    if (!pending) return { kind: 'up-to-date', currentVersion };
    const target = await deps.linuxInfo();
    if (target.managed) {
      return {
        kind: 'available-linux',
        currentVersion,
        version: pending.version,
        notes: pending.notes,
      };
    }
    if (target.os === 'linux') {
      return { kind: 'unsupported', currentVersion, reason: target.reason };
    }
    return {
      kind: 'available-stock',
      currentVersion,
      version: pending.version,
      notes: pending.notes,
      install: pending.install,
    };
  } catch (error) {
    return { kind: 'error', currentVersion, message: friendlyUpdateError(error) };
  }
}

export function friendlyUpdateError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error ?? '');
  if (/signature|verif|tamper|unsigned|forg/i.test(message)) {
    return 'The update signature did not verify. The update was blocked for your safety.';
  }
  if (/network|offline|fetch|connect|dns|timeout|timed out|could not reach|http|url|manifest/i.test(message)) {
    return 'Could not reach the update server. Check your connection, then retry.';
  }
  if (/cancel/i.test(message)) return 'The update was cancelled before it finished.';
  if (/permission|denied|access|readonly|read-only/i.test(message)) {
    return 'The update could not replace the app files (permission denied). Install the new version from GitHub Releases instead.';
  }
  return message || 'The update could not be installed. Retry, or install from GitHub Releases.';
}

/** 0..1 download fraction, or null when the total size is unknown. */
export function progressFraction(downloaded: number, total?: number | null): number | null {
  if (!total || total <= 0) return null;
  return Math.min(1, Math.max(0, downloaded / total));
}
