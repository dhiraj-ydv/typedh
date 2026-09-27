import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createNativeApi } from '../src/native-api.ts';
import {
  checkForUpdates,
  friendlyUpdateError,
  progressFraction,
  isDesktop,
} from '../src/updates.ts';

function deps(overrides = {}) {
  return {
    getAppVersion: async () => '0.1.2',
    checkPlugin: async () => null,
    linuxInfo: async () => ({ os: 'other', managed: false, prefix: null, reason: '' }),
    ...overrides,
  };
}

test('no newer release reports up-to-date', async () => {
  const result = await checkForUpdates(deps());
  assert.deepEqual(result, { kind: 'up-to-date', currentVersion: '0.1.2' });
});

test('newer release on Windows/macOS uses the stock installer flow', async () => {
  const install = async () => {};
  const result = await checkForUpdates(
    deps({
      checkPlugin: async () => ({ version: '0.2.0', notes: 'Faster.', install }),
      linuxInfo: async () => ({ os: 'windows', managed: false, prefix: null, reason: '' }),
    }),
  );
  assert.equal(result.kind, 'available-stock');
  assert.equal(result.version, '0.2.0');
  assert.equal(result.notes, 'Faster.');
  assert.equal(result.install, install);
});

test('newer release on a managed Linux prefix uses the backend flow', async () => {
  const result = await checkForUpdates(
    deps({
      checkPlugin: async () => ({ version: '0.2.0', notes: '', install: async () => {} }),
      linuxInfo: async () => ({
        os: 'linux',
        managed: true,
        prefix: '/home/u/.local',
        reason: '',
      }),
    }),
  );
  assert.equal(result.kind, 'available-linux');
  assert.equal(result.version, '0.2.0');
});

test('system Linux installs are unsupported, not installable', async () => {
  const result = await checkForUpdates(
    deps({
      checkPlugin: async () => ({ version: '0.2.0', notes: '', install: async () => {} }),
      linuxInfo: async () => ({ os: 'linux', managed: false, prefix: null, reason: 'Use pacman.' }),
    }),
  );
  assert.deepEqual(result, {
    kind: 'unsupported',
    currentVersion: '0.1.2',
    reason: 'Use pacman.',
  });
});

test('check failures surface friendly messages with the current version', async () => {
  const offline = await checkForUpdates(
    deps({
      checkPlugin: async () => {
        throw new Error('failed to fetch latest.json: network unreachable');
      },
    }),
  );
  assert.equal(offline.kind, 'error');
  assert.equal(offline.currentVersion, '0.1.2');
  assert.match(offline.message, /connection/i);

  const noVersion = await checkForUpdates(
    deps({
      getAppVersion: async () => {
        throw new Error('invoke not available');
      },
    }),
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

test('new native commands use the expected payload shapes', async () => {
  const calls = [];
  const api = createNativeApi(async (command, args) => {
    calls.push([command, args]);
    if (command === 'app_version') return '0.1.2';
    return { os: 'linux', managed: true, prefix: '/home/u/.local', reason: '' };
  });
  assert.equal(await api.appVersion(), '0.1.2');
  assert.deepEqual(await api.linuxInstallInfo(), {
    os: 'linux',
    managed: true,
    prefix: '/home/u/.local',
    reason: '',
  });
  await api.installLinuxUpdate('0.2.0');
  assert.deepEqual(calls, [
    ['app_version', undefined],
    ['linux_install_info', undefined],
    ['install_linux_update', { expectedVersion: '0.2.0' }],
  ]);
});
