import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
const read = path => readFileSync(new URL('../' + path, import.meta.url), 'utf8');
const version = JSON.parse(read('package.json')).version;
const versions = [
  JSON.parse(read('package-lock.json')).version,
  JSON.parse(read('src-tauri/tauri.conf.json')).version,
  read('src-tauri/Cargo.toml').match(/^version = "([^"]+)"/m)?.[1],
  read('src-tauri/Cargo.lock').match(/name = "typedh"\r?\nversion = "([^"]+)"/)?.[1],
  read('packaging/arch/PKGBUILD').match(/^pkgver=(.+)$/m)?.[1],
];
for (const value of versions) assert.equal(value, version, 'Application versions disagree');
if (process.env.GITHUB_REF_TYPE === 'tag') {
  assert.match(process.env.GITHUB_REF_NAME, /^v\d+\.\d+\.\d+$/);
  assert.equal(process.env.GITHUB_REF_NAME, 'v' + version);
}
console.log('Application version verified:', version);
