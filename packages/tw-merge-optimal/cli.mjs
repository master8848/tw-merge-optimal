import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const WASI_RUNNER = fileURLToPath(new URL('./wasi-runner.mjs', import.meta.url));
const WASM_BIN = join(THIS_DIR, 'bin', 'twm-gen.wasm');

export const USAGE = `tw-merge-optimal

Generates a dependency-free twMerge/twJoin bundle from your Tailwind class usage.

USAGE
  tw-merge-optimal [options] <globs-or-paths...>

OPTIONS
  --css <file>     extra @utility/@theme CSS to extend the design system
  --config <file>  JSON config (classGroups / conflictingClassGroups) to extend
                   the design system
  --out <file>     write the generated JS bundle to <file> (default: stdout)
  --prefix <p>     only treat classes with the \`p:\` prefix as Tailwind classes
  --no-patterns    emit only the scanned classes (smaller bundle; classes the
                   scanner missed pass through unmerged — default is full
                   pattern-table resolution, so unseen classes still merge)
  --check          report conflicts among used classes; exit 1 if any exist
  -h, --help       show this help

ENVIRONMENT
  TWM_GEN_WASM     path to a twm-gen WASI module (bin/twm-gen.wasm is used
                   automatically if present — build it with: npm run build:wasm)
  TWM_GEN_BIN      path to a native twm-gen binary (fallback when no WASM is
                   available; resolved like before: workspace target/, or the
                   postinstall download under bin/)`;

function workspaceRoot() {
    let dir = THIS_DIR;
    for (;;) {
        const cargo = join(dir, 'Cargo.toml');
        if (existsSync(cargo)) {
            const contents = readFileSync(cargo, 'utf8');
            if (/\[workspace\]/.test(contents) && /twm-gen/.test(contents)) {
                return dir;
            }
        }
        const parent = dirname(dir);
        if (parent === dir) return null;
        dir = parent;
    }
}

export function findBinary() {
    if (process.env.TWM_GEN_BIN) return process.env.TWM_GEN_BIN;
    const root = workspaceRoot();
    if (root) {
        for (const profile of ['release', 'debug']) {
            const bin = join(root, 'target', profile, 'twm-gen');
            if (existsSync(bin)) return bin;
        }
    }
    // Prebuilt binary downloaded by the postinstall script
    // (node_modules/tw-merge-optimal/bin).
    const ext = process.platform === 'win32' ? '.exe' : '';
    const downloaded = join(THIS_DIR, 'bin', `twm-gen-${process.platform}-${process.arch}${ext}`);
    if (existsSync(downloaded)) return downloaded;
    throw new Error(
        'tw-merge-optimal: cannot locate a twm-gen engine.\n' +
            '  The WASM build (bin/twm-gen.wasm) is preferred — build it with:\n' +
            '    npm run build:wasm\n' +
            '  or use a native binary via one of:\n' +
            '  - set TWM_GEN_BIN=/path/to/twm-gen\n' +
            '  - re-run npm install (postinstall downloads a prebuilt binary from GitHub Releases)\n' +
            '  - build it yourself: cargo build -p twm-gen --release'
    );
}

export function findEngine() {
    const wasmPath = process.env.TWM_GEN_WASM || (existsSync(WASM_BIN) ? WASM_BIN : null);
    if (wasmPath) return { kind: 'wasm', path: wasmPath };
    return { kind: 'native', path: findBinary() };
}

export function runEngine(args, options = {}) {
    const engine = findEngine();
    return runWith(engine, args, options);
}

export function runWith(engine, args, options = {}) {
    if (engine.kind === 'wasm') {
        const flags =
            Number(process.versions.node.split('.')[0]) <= 18
                ? ['--experimental-wasi-unstable-preview1']
                : [];
        return spawnSync(process.execPath, [...flags, WASI_RUNNER, engine.path, ...args], {
            encoding: 'utf8',
            ...options,
        });
    }
    return spawnSync(engine.path, args, { encoding: 'utf8', ...options });
}

export function defaultOut() {
    return fileURLToPath(new URL('./generated.mjs', import.meta.url));
}

export const DEFAULT_SOURCES = [
    'src/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
    'app/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
    'pages/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
    'components/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
];

export const IMPORT_IDS = [
    'tw-merge-optimal',
    'tw-merge-optimal/index.mjs',
    'tw-merge-optimal/generated.mjs',
];

export function resolveSources(options = {}) {
    const sources = options.include ?? DEFAULT_SOURCES;
    return sources.map((s) => (s.startsWith('.') || s.startsWith('/') ? s : `./${s}`));
}

export function resolveOutFile(options = {}) {
    if (options.outFile) return resolve(process.cwd(), options.outFile);
    return join(process.cwd(), '.tw-merge-optimal', 'generated.mjs');
}

export function generate(options = {}) {
    const {
        sources = [],
        css,
        config,
        out,
        prefix,
        patterns,
        check,
        engine,
    } = options;

    const args = [];
    if (css) args.push('--css', css);
    if (out) args.push('--out', out);
    if (prefix) args.push('--prefix', prefix);
    // Patterns (unseen classes still resolve) are the default; opt out
    // explicitly for a smaller bundle.
    if (patterns === false) args.push('--no-patterns');
    if (check) args.push('--check');
    args.push(...sources);

    if (out) mkdirSync(dirname(out), { recursive: true });

    let configFile = null;
    try {
        if (config) {
            const dir = join(process.cwd(), '.tw-merge-optimal');
            mkdirSync(dir, { recursive: true });
            configFile = join(dir, `config-${process.pid}.json`);
            writeFileSync(configFile, JSON.stringify(config));
            args.push('--config', configFile);
        }

        const result = engine ? runWith(engine, args) : runEngine(args);
        const stderr = result.stderr ?? '';
        const status = result.status ?? -1;

        if (status !== 0) {
            throw new Error(
                `tw-merge-optimal: twm-gen failed (exit ${status})\n${stderr.trim()}`
            );
        }

        let bytes = null;
        if (out) {
            const m = stderr.match(/wrote \S+ \((\d+) bytes/);
            bytes = m ? Number(m[1]) : 0;
        }

        return {
            bundle: out ? null : (result.stdout ?? ''),
            bytes,
            stderr,
            status,
        };
    } finally {
        if (configFile) {
            try {
                unlinkSync(configFile);
            } catch {}
        }
    }
}

export function runCli(argv = process.argv.slice(2)) {
    if (argv.includes('-h') || argv.includes('--help')) {
        process.stdout.write(USAGE + '\n');
        return 0;
    }
    const result = runEngine(argv, { stdio: 'inherit' });
    return result.status ?? 1;
}

export default runCli;
