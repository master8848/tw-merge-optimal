// postinstall: download the prebuilt twm-gen binary for this platform from
// GitHub Releases so users never need a Rust toolchain. Skips when a binary
// is already available (TWM_GEN_BIN env, or a cargo-built binary in the
// source workspace). Failures warn and exit 0 — `findBinary` reports the
// actionable error at build time.
import { existsSync, mkdirSync, createWriteStream } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO = process.env.TWM_GEN_REPO || 'master8848/tw-merge-optimal';
const VERSION = process.env.TWM_GEN_VERSION || 'latest';

const PLATFORM = {
    darwin: 'darwin',
    linux: 'linux',
    win32: 'win32',
}[process.platform];
const ARCH = {
    x64: 'x64',
    arm64: 'arm64',
}[process.arch];

function artifactName() {
    const base = `twm-gen-${PLATFORM}-${ARCH}`;
    return process.platform === 'win32' ? `${base}.exe` : base;
}

function cargoBinary() {
    // In the source workspace a cargo-built binary takes precedence; outside
    // a workspace (normal npm installs) this is skipped.
    const root = join(THIS_DIR, '..', '..', 'Cargo.toml');
    if (!existsSync(root)) return null;
    try {
        execFileSync('cargo', ['build', '-p', 'twm-gen', '--release'], {
            cwd: join(THIS_DIR, '..', '..'),
            stdio: 'pipe',
        });
        for (const profile of ['release', 'debug']) {
            const bin = join(THIS_DIR, '..', '..', 'target', profile, 'twm-gen');
            if (existsSync(bin)) return bin;
        }
    } catch {
        // cargo unavailable — fall through to the download path
    }
    return null;
}

function download(res, target) {
    return new Promise((resolve, reject) => {
        const ws = createWriteStream(target);
        const reader = res.body.getReader();
        const pump = () =>
            reader
                .read()
                .then(({ done, value }) => {
                    if (done) {
                        ws.end();
                        resolve();
                    } else {
                        ws.write(Buffer.from(value));
                        pump();
                    }
                })
                .catch(reject);
        ws.on('error', reject);
        pump();
    });
}

async function main() {
    const fromCargo = cargoBinary();
    if (fromCargo) {
        console.log(`tw-merge-optimal: using workspace binary ${fromCargo}`);
        process.exit(0);
    }
    if (process.env.TWM_GEN_BIN && existsSync(process.env.TWM_GEN_BIN)) {
        console.log(`tw-merge-optimal: TWM_GEN_BIN set to ${process.env.TWM_GEN_BIN}`);
        process.exit(0);
    }

    if (!PLATFORM || !ARCH) {
        console.warn(
            `tw-merge-optimal: no prebuilt binary for ${process.platform}/${process.arch}; ` +
                'set TWM_GEN_BIN to a twm-gen binary or build with `cargo build -p twm-gen --release`.'
        );
        process.exit(0);
    }
    if (process.env.TWM_NO_DOWNLOAD) process.exit(0);

    const binDir = join(THIS_DIR, 'bin');
    const target = join(binDir, artifactName());
    if (existsSync(target)) {
        console.log(`tw-merge-optimal: using cached binary ${target}`);
        process.exit(0);
    }

    const url = `https://github.com/${REPO}/releases/${VERSION}/download/${artifactName()}`;
    try {
        mkdirSync(binDir, { recursive: true });
        console.log(`tw-merge-optimal: downloading ${url}`);
        const res = await fetch(url);
        if (!res.ok || !res.body) {
            throw new Error(`HTTP ${res.status}`);
        }
        await download(res, target);
        if (process.platform !== 'win32') {
            execFileSync('chmod', ['+x', target], { stdio: 'pipe' });
        }
        console.log(`tw-merge-optimal: installed prebuilt ${artifactName()}`);
    } catch (err) {
        console.warn(
            `tw-merge-optimal: binary download failed (${err.message}).\n` +
                `  The twm-gen binary is needed when building. Fix options:\n` +
                `  - set TWM_GEN_BIN=/path/to/twm-gen\n` +
                `  - or install Rust once: cargo build -p twm-gen --release\n` +
                `  - or check the release exists at https://github.com/${REPO}/releases`
        );
        process.exit(0);
    }
}

main();
