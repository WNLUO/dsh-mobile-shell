// 把对应平台上已经构建好的 Tauri 裸产物打成按平台的 npm 包，供插件的远程电脑桌面窗口使用。
//
// Tauri 依赖各平台的系统 WebView，不能像 Go 那样在一台机器上交叉编译五个平台；每个平台都要在
// 对应系统上编好，这个脚本只负责收产物：从 Tauri/src-tauri/target/release/ 取可执行文件，
// macOS 从 target/release/bundle/macos/ 取整份 .app，复制到 build/desktop-shell/<平台>/，
// 写好 package.json 与 NOTICE，并保住可执行权限。产物不进仓库。
//
// 按平台发可选依赖，走的还是插件自己那条包管理器链路（国内镜像、完整性校验、断点重试都是现成的），
// 不做运行时下载；装不上就没有桌面窗口，插件回退系统浏览器，其余功能一点不受影响。
//
// 版本单独走 Tauri/shell-version.json：壳没改就不抬版本，插件更新不会重下平台包。
//
//   node tools/build-desktop-shell.mjs darwin-arm64   # 一次只收一个平台的产物

import { chmodSync, cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const shellDir = join(root, 'Tauri');
const releaseDir = join(shellDir, 'src-tauri', 'target', 'release');
const macosBundleDir = join(releaseDir, 'bundle', 'macos');
const outRoot = join(root, 'build', 'desktop-shell');
const BINARY = 'dsh-mobile-shell';

/** 平台包里的启动路径与 package.json 的 os/cpu 一起定死；插件按同一份路径 spawn。 */
const TARGETS = [
  { id: 'darwin-arm64', os: 'darwin', cpu: 'arm64', binary: `${BINARY}.app/Contents/MacOS/${BINARY}` },
  { id: 'darwin-x64', os: 'darwin', cpu: 'x64', binary: `${BINARY}.app/Contents/MacOS/${BINARY}` },
  { id: 'linux-x64', os: 'linux', cpu: 'x64', binary: BINARY },
  { id: 'linux-arm64', os: 'linux', cpu: 'arm64', binary: BINARY },
  { id: 'win32-x64', os: 'win32', cpu: 'x64', binary: `${BINARY}.exe` },
];

const NOTICE = `这个程序（远程电脑的桌面显示壳）是 DSH Mobile 的一部分，MIT 许可。

它随插件按平台的可选依赖分发，只负责显示远程电脑的一次性网页入口，不是单独安装的应用。

它基于 Tauri 2 构建，运行时链接 wry、tao 与各平台的系统 WebView；第三方组件的许可与版权声明以各自项目为准，WebView 由操作系统提供。
`;

function fail(message) {
  console.error(`build-desktop-shell: ${message}`);
  process.exit(1);
}

function readShellVersion() {
  const path = join(shellDir, 'shell-version.json');
  if (!existsSync(path)) fail(`缺 ${path}`);
  const { version } = JSON.parse(readFileSync(path, 'utf8'));
  if (!/^\d+\.\d+\.\d+$/.test(String(version))) fail('shell-version.json 的 version 必须是 x.y.z');
  return version;
}

/** .app 目录名跟 productName 走，包内统一叫 dsh-mobile-shell.app。 */
function macAppSource() {
  if (!existsSync(macosBundleDir)) fail(`缺 ${macosBundleDir}，先在 macOS 上编出 .app`);
  const apps = readdirSync(macosBundleDir, { withFileTypes: true })
    .filter(entry => entry.isDirectory() && entry.name.endsWith('.app'))
    .map(entry => entry.name);
  if (apps.length !== 1) fail(`${macosBundleDir} 里应当只有一份 .app，实际 ${apps.length} 份`);
  return join(macosBundleDir, apps[0]);
}

function copyArtifact(target, dir) {
  if (target.os === 'darwin') {
    cpSync(macAppSource(), join(dir, `${BINARY}.app`), { recursive: true });
  } else {
    const source = join(releaseDir, target.binary);
    if (!existsSync(source)) fail(`缺 ${source}，先在当前平台编出 ${BINARY}`);
    cpSync(source, join(dir, target.binary));
  }
  const executable = join(dir, target.binary);
  if (!existsSync(executable)) fail(`缺 ${executable}；tauri.conf.json 的 mainBinaryName 应为 ${BINARY}`);
  if (target.os !== 'win32') chmodSync(executable, 0o755);
  return executable;
}

function packageFiles(target) {
  return target.os === 'darwin' ? [`${BINARY}.app`, target.binary, 'NOTICE'] : [target.binary, 'NOTICE'];
}

function writePackage(target, dir, version) {
  writeFileSync(
    join(dir, 'package.json'),
    `${JSON.stringify(
      {
        name: `dsh-mobile-shell-${target.id}`,
        version,
        description: `远程电脑的桌面显示壳（${target.id}）`,
        os: [target.os],
        cpu: [target.cpu],
        files: packageFiles(target),
        license: 'MIT',
        private: false,
        repository: {
          type: 'git',
          url: 'git+https://github.com/WNLUO/dsh-mobile-shell.git',
        },
        publishConfig: {
          registry: 'https://registry.npmjs.org/',
          access: 'public',
        },
      },
      null,
      2,
    )}\n`,
  );
  writeFileSync(join(dir, 'NOTICE'), NOTICE);
}

const only = process.argv[2];
const target = TARGETS.find(item => item.id === only);
if (!target) fail(`要指定一个平台：${TARGETS.map(item => item.id).join(' / ')}`);

const version = readShellVersion();
const dir = join(outRoot, target.id);
rmSync(dir, { recursive: true, force: true });
mkdirSync(dir, { recursive: true });
const executable = copyArtifact(target, dir);
writePackage(target, dir, version);
console.log(`  ${target.id.padEnd(14)} ${(statSync(executable).size / 1048576).toFixed(1)} MB  ${target.binary}`);
console.log(`产物：${dir}（版本 ${version}）`);
console.log('五个平台包齐了以后跑 node tools/publish-desktop-shell.mjs。');
