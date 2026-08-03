import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { tmpdir } from 'node:os';
import { findBinary } from '../cli.mjs';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));

let binaryMemo = null;

export function ensureBinary() {
    if (binaryMemo) return binaryMemo;
    try {
        binaryMemo = findBinary();
        return binaryMemo;
    } catch {
        let root = THIS_DIR;
        for (;;) {
            if (existsSync(join(root, 'Cargo.toml'))) break;
            const parent = dirname(root);
            if (parent === root) throw new Error('cannot find repo root');
            root = parent;
        }
        execFileSync('cargo', ['build', '-p', 'twm-gen', '--release'], {
            cwd: root,
            stdio: 'inherit',
        });
        binaryMemo = findBinary();
        return binaryMemo;
    }
}

const FIXTURE_SOURCE =
    "const x = 'px-2 py-1 p-3 bg-red-500 text-2xl';\n" +
    "const y = 'bg-blue-500 hover:bg-red-500 md:text-3xl';\n";

export function makeFixture() {
    const dir = join(tmpdir(), `twmo-${process.pid}-${Math.random().toString(36).slice(2)}`);
    const srcDir = join(dir, 'src');
    mkdirSync(srcDir, { recursive: true });
    const file = join(srcDir, 'app.tsx');
    writeFileSync(file, FIXTURE_SOURCE);
    return { dir, srcDir, file };
}

export function importPath(path) {
    return import(pathToFileURL(path).href);
}
