import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));

export const USAGE = `tw-merge-optimal

Generates a dependency-free twMerge/twJoin bundle from your Tailwind class usage.

USAGE
  tw-merge-optimal [options] <globs-or-paths...>

OPTIONS
  --css <file>     extra @utility/@theme CSS to extend the design system
  --out <file>     write the generated JS bundle to <file> (default: stdout)
  --prefix <p>     only treat classes with the \`p:\` prefix as Tailwind classes
  --patterns       treat the paths as glob patterns (default: literal paths)
  --check          report conflicts among used classes; exit 1 if any exist
  -h, --help       show this help

ENVIRONMENT
  TWM_GEN_BIN      path to the twm-gen binary (overrides auto-detection)

The binary is resolved from the workspace Cargo.toml (target/release or
target/debug); if missing, build it with: cargo build -p twm-gen --release`;

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
    throw new Error(
        'tw-merge-optimal: cannot locate the twm-gen binary. ' +
            'Build it with `cargo build -p twm-gen --release`, or set TWM_GEN_BIN to its path.'
    );
}

export function defaultOut() {
    return fileURLToPath(new URL('./generated.mjs', import.meta.url));
}

export function generate(options = {}) {
    const {
        sources = [],
        css,
        out,
        prefix,
        patterns,
        check,
        binary = findBinary(),
    } = options;

    const args = [];
    if (css) args.push('--css', css);
    if (out) args.push('--out', out);
    if (prefix) args.push('--prefix', prefix);
    if (patterns) args.push('--patterns');
    if (check) args.push('--check');
    args.push(...sources);

    const result = spawnSync(binary, args, { encoding: 'utf8' });
    const stderr = result.stderr ?? '';
    const status = result.status ?? -1;

    if (status !== 0) {
        throw new Error(
            `tw-merge-optimal: twm-gen failed (exit ${status})\n${stderr.trim()}`
        );
    }

    let bytes = null;
    if (out) {
        const m = stderr.match(/wrote \S+ \((\d+) bytes\)/);
        bytes = m ? Number(m[1]) : 0;
    }

    return {
        bundle: out ? null : (result.stdout ?? ''),
        bytes,
        stderr,
        status,
    };
}

export function runCli(argv = process.argv.slice(2)) {
    if (argv.includes('-h') || argv.includes('--help')) {
        process.stdout.write(USAGE + '\n');
        return 0;
    }
    const binary = findBinary();
    const result = spawnSync(binary, argv, { stdio: 'inherit' });
    return result.status ?? 1;
}

export default runCli;
