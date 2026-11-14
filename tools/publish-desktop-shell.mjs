// 发布远程电脑桌面显示壳的五个平台包，并打印插件 optionalDependencies 要登记的五条依赖。
//
// 顺序不能反：平台包先上 npm，插件才把版本号登记进 optionalDependencies —— 登记早了会撞
// pnpm 的 --frozen-lockfile（锁文件里没有这几条，本地与 CI 都装不动）。插件更新锁文件后
// 才轮到发主插件包，这样主包带的是已经存在于 npm 上的依赖。插件清单在插件仓库维护，本脚本
// 只给出要写入的包名与版本号。
//
//   node tools/publish-desktop-shell.mjs --dry-run     # 只看要发什么
//   node tools/publish-desktop-shell.mjs               # 真发五个平台包
//   node tools/publish-desktop-shell.mjs --wire        # 只打印要登记的五条依赖
//
// 版本号来自 Tauri/shell-version.json，独立于插件版本：壳没改就别抬版本，插件每次更新
// 不会重下平台包。

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const builtRoot = join(root, 'build', 'desktop-shell');
const shellVersion = JSON.parse(readFileSync(join(root, 'Tauri', 'shell-version.json'), 'utf8')).version;

const PLATFORMS = ['darwin-arm64', 'darwin-x64', 'linux-x64', 'linux-arm64', 'win32-x64'];
const dryRun = process.argv.includes('--dry-run');
const wireOnly = process.argv.includes('--wire');

function fail(message) {
  console.error(`publish-desktop-shell: ${message}`);
  process.exit(1);
}

function wire() {
  console.log(`需要写进插件 optionalDependencies 的五个平台包（版本 ${shellVersion}）：`);
  for (const platform of PLATFORMS) console.log(`  dsh-mobile-shell-${platform}@${shellVersion}`);
}

if (wireOnly) {
  wire();
  process.exit(0);
}

for (const platform of PLATFORMS) {
  const dir = join(builtRoot, platform);
  if (!existsSync(join(dir, 'package.json'))) fail(`缺 ${dir}，先在各平台跑 node tools/build-desktop-shell.mjs <平台>`);
  const args = ['publish', '--access', 'public'];
  if (dryRun) args.push('--dry-run');
  console.log(`> ${platform} ${shellVersion}`);
  try {
    execFileSync('npm', args, { cwd: dir, stdio: 'inherit' });
  } catch {
    fail(`${platform} 发布失败`);
  }
}

if (!dryRun) wire();
