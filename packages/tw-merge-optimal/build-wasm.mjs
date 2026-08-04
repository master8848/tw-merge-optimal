// Local build script: compile twm-gen to wasm32-wasip1 and install the
// artifact at bin/twm-gen.wasm so the package runs fully under Node's built-in
// WASI runtime — one artifact, every platform, no native binary needed.
// (The GitHub Releases pipeline is intentionally untouched for now.)
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url)); // packages/tw-merge-optimal
const ROOT = join(HERE, '..', '..');
const TARGET = join(ROOT, 'target', 'wasm32-wasip1', 'release', 'twm-gen.wasm');
const DEST = join(HERE, 'bin', 'twm-gen.wasm');

try {
    execFileSync('rustup', ['target', 'add', 'wasm32-wasip1'], { stdio: 'inherit' });
} catch {
    // rustup unavailable — assume the target is already installed
}
execFileSync('cargo', ['build', '-p', 'twm-gen', '--target', 'wasm32-wasip1', '--release'], {
    cwd: ROOT,
    stdio: 'inherit',
});
mkdirSync(dirname(DEST), { recursive: true });
copyFileSync(TARGET, DEST);
console.log(`tw-merge-optimal: wasm build installed at ${DEST}`);
