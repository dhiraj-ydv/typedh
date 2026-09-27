import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createNativeApi } from '../src/native-api.ts';
import {
  checkForUpdates,
  friendlyUpdateError,
  progressFraction,
  isDesktop,
  isLinux,
  LINUX_PACMAN_MESSAGE,
} from '../src/updates.ts';

function deps(overrides = {}) {
  return {
    getAppVersion: async () => '0.1.2',
    checkPlugin: async () => null,
    ...overrides,
  };
}

function withLinuxUserAgent(value, fn) {
  const hadNavigator = 'navigator' in globalThis;
  const previous = globalThis.navigator;
  if (value === null) {
    // @ts-ignore - test helper removes the browser global.
    delete globalThis.navigator;
  } else {
    // @ts-ignore - test helper installs a minimal navigator stub.
    globalThis.navigator = { userAgent: value, platform: '' };
  }
  try {
    return fn();
  } finally {
    if (hadNavigator) globalThis.navigator = previous;
    else delete globalThis.navigator;
  }
}

test('no newer release reports up-to-date', async () => {
  const result = await withLinuxUserAgent(null, () => checkForUpdates(deps()));
  assert.deepEqual(result, { kind: 'up-to-date', currentVersion: '0.1.2' });
});

test('newer release on Windows/macOS uses the stock installer flow', async () => {
  const install = async () => {};
  const result = await withLinuxUserAgent(null, () =>
    checkForUpdates(
      deps({
        checkPlugin: async () => ({ version: '0.2.0', notes: 'Faster.', install }),
      }),
    ),
  );
  assert.equal(result.kind, 'available-stock');
  assert.equal(result.version, '0.2.0');
  assert.equal(result.notes, 'Faster.');
  assert.equal(result.install, install);
});

test('Linux pacman installs are unsupported with pacman instructions', async () => {
  const result = await withLinuxUserAgent('Mozilla/5.0 (X11; Linux x86_64)', () =>
    checkForUpdates(
      deps({
        checkPlugin: async () => ({ version: '0.2.0', notes: '', install: async () => {} }),
      }),
    ),
  );
  assert.deepEqual(result, {
    kind: 'unsupported',
    currentVersion: '0.1.2',
    reason: LINUX_PACMAN_MESSAGE,
  });
  assert.match(result.reason, /pacman -U/);
});

test('check failures surface friendly messages with the current version', async () => {
  const offline = await withLinuxUserAgent(null, () =>
    checkForUpdates(
      deps({
        checkPlugin: async () => {
          throw new Error('failed to fetch latest.json: network unreachable');
        },
      }),
    ),
  );
  assert.equal(offline.kind, 'error');
  assert.equal(offline.currentVersion, '0.1.2');
  assert.match(offline.message, /connection/i);

  const noVersion = await withLinuxUserAgent(null, () =>
    checkForUpdates(
      deps({
        getAppVersion: async () => {
          throw new Error('invoke not available');
        },
      }),
    ),
  );
  assert.equal(noVersion.kind, 'error');
  assert.equal(noVersion.currentVersion, null);
});

test('friendlyUpdateError maps tamper, network, cancel, and permission failures', () => {
  assert.match(friendlyUpdateError(new Error('signature did not verify')), /blocked for your safety/);
  assert.match(friendlyUpdateError(new Error('timeout after 30s')), /connection/i);
  assert.match(friendlyUpdateError('cancelled by user'), /cancelled/i);
  assert.match(friendlyUpdateError(new Error('permission denied writing binary')), /permission denied/i);
  assert.equal(friendlyUpdateError(new Error('weird')), 'weird');
  assert.match(friendlyUpdateError(undefined), /GitHub Releases/);
});

test('progressFraction clamps and tolerates unknown totals', () => {
  assert.equal(progressFraction(50, 100), 0.5);
  assert.equal(progressFraction(200, 100), 1);
  assert.equal(progressFraction(10, 0), null);
  assert.equal(progressFraction(10, undefined), null);
});

test('isDesktop is false outside the Tauri webview', () => {
  assert.equal(isDesktop(), false);
});

test('isLinux detects Linux user agents and tolerates missing navigator', () => {
  assert.equal(
    withLinuxUserAgent('Mozilla/5.0 (X11; Linux x86_64)', () => isLinux()),
    true,
  );
  assert.equal(
    withLinuxUserAgent('Mozilla/5.0 (Windows NT 10.0; Win64; x64)', () => isLinux()),
    false,
  );
  assert.equal(
    withLinuxUserAgent(null, () => isLinux()),
    false,
  );
});

test('native commands use the expected payload shapes', async () => {
  const calls = [];
  const api = createNativeApi(async (command, args) => {
    calls.push([command, args]);
    if (command === 'app_version') return '0.1.2';
    return [];
  });
  assert.equal(await api.appVersion(), '0.1.2');
  assert.deepEqual(calls, [['app_version', undefined]]);
});
