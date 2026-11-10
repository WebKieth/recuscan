import fs from 'node:fs';
import path from 'node:path';

const target = process.env.BUILD_TARGET;
const extensions = process.env.PACKAGE_EXTENSIONS?.split(',');
if (!target || !extensions?.length) {
  throw new Error('BUILD_TARGET and PACKAGE_EXTENSIONS are required');
}

const root = path.join('src-tauri', 'target', target, 'release', 'bundle');
const output = 'release';
fs.mkdirSync(output, { recursive: true });
const found = new Set();

function collect(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const source = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      // Do not descend into macOS application bundles or their frameworks.
      if (!entry.name.endsWith('.app')) collect(source);
    } else if (entry.isFile()) {
      const extension = extensions.find((suffix) => entry.name.endsWith(suffix));
      if (!extension) continue;
      const destination = path.join(output, entry.name);
      if (fs.existsSync(destination)) throw new Error(`Duplicate package: ${entry.name}`);
      fs.copyFileSync(source, destination);
      found.add(extension);
      console.log(`Collected ${entry.name}`);
    }
  }
}

collect(root);
for (const extension of extensions) {
  if (!found.has(extension)) throw new Error(`Missing package: ${extension}`);
}
