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
  | { kind: 'unsupported'; currentVersion: string; reason: string }
  | { kind: 'error'; currentVersion: string | null; message: string };

export function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

export function isLinux(): boolean {
  if (typeof navigator === 'undefined') return false;
  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const userAgent = nav.userAgent ?? '';
  // Node.js exposes a navigator global (userAgent "Node.js/..."); never treat
  // the test/runtime host itself as a Linux desktop. Real Linux browsers and
  // WebViews carry "Linux" in the userAgent.
  if (/node\.js/i.test(userAgent)) return false;
  const platform = nav.userAgentData?.platform ?? '';
  return /linux/i.test(platform) || /linux/i.test(userAgent);
}

export const LINUX_PACMAN_MESSAGE =
  'Linux installs update with sudo pacman -U, not in-app updates. Download the newer .pkg.tar.zst from GitHub Releases and run sudo pacman -U on it; pacman -Syu alone will not pick it up.';

/**
 * Check for updates. Windows and macOS use the signed stock Tauri updater
 * flow. Linux (Arch `.pkg.tar.zst` via `pacman -U`, issue #17) has no
 * in-app updater and is reported as unsupported with pacman instructions.
 */
export async function checkForUpdates(deps: UpdateFlowDeps): Promise<UpdateCheck> {
  let currentVersion: string | null = null;
  try {
    currentVersion = await deps.getAppVersion();
    if (isLinux()) {
      return { kind: 'unsupported', currentVersion, reason: LINUX_PACMAN_MESSAGE };
    }
    const pending = await deps.checkPlugin();
    if (!pending) return { kind: 'up-to-date', currentVersion };
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
