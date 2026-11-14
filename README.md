# 远程电脑桌面显示壳

DSH Mobile「远程电脑」功能的桌面显示壳，基于 Tauri 2。这个仓库只放显示组件的源码、五个平台的构建与发布配置。

它不是独立应用，也不做安装器、不注册自定义协议、不做单实例：它作为 `dsh-mobile-plugin` 随包按平台分发的显示组件，只用来显示「远程电脑」的一次性网页入口。

插件在 B 电脑的控制台点「在桌面端打开」时，先向本机 `POST /api/relay/remote-computers/open-shell` 拿到 `{ ok, url, expiresAt }`，再 spawn 平台包里的可执行文件：

```sh
dsh-mobile-shell --remote-url=<一次性地址> --device-name=<设备名>
```

一次点击启动一个进程、开一个窗口，窗口标题带设备名；多台设备可以同时多开，互不干扰。机器上没装对应平台包时接口返回 `SHELL_UNAVAILABLE`，插件自动回退到系统浏览器路径，远程电脑功能照常可用。

窗口只加载 `http://127.0.0.1:<端口>` 的一次性入口，导航只允许同源（127.0.0.1 且同一端口）；跨来源、新窗口、非 http(s) 或带账号密码的链接一律拦下，外链交给系统浏览器打开且不带本次会话。凭据不落盘，窗口关闭即结束本次访问。

## 五个平台包

平台包 `dsh-mobile-shell-<os>-<cpu>` 走 npm 的可选依赖链路分发，不做运行时下载。版本取 `Tauri/shell-version.json`，独立于插件版本：壳没改就不抬版本，插件更新不会重下平台包。五个平台 id：

- `darwin-arm64` / `darwin-x64`：`dsh-mobile-shell.app/Contents/MacOS/dsh-mobile-shell`
- `linux-x64` / `linux-arm64`：`dsh-mobile-shell`
- `win32-x64`：`dsh-mobile-shell.exe`

可执行文件统一名 `dsh-mobile-shell`（`tauri.conf.json` 的 `mainBinaryName`），由 Tauri CLI 在构建时重命名。各平台包用 `os` / `cpu` 限定，包管理器只会装当前机器那一份。

## 构建

Tauri 依赖各平台的系统 WebView，不能在一台机器上交叉编译五个平台。每个平台都要在对应系统上编好裸应用产物，产物统一落在 `Tauri/src-tauri/target/release/`：

```sh
# 在对应平台的机器上，Rust ≥ 1.90
cd Tauri
npx --yes @tauri-apps/cli@2 build --bundles app     # macOS：只出 .app，不打 dmg
npx --yes @tauri-apps/cli@2 build --no-bundle       # Windows / Linux：只出可执行文件，不打安装器
cd ..
node tools/build-desktop-shell.mjs <平台 id>         # 收产物，打成 build/desktop-shell/<平台>/
```

Tauri CLI 只在构建时临时用一次，不写进 `Tauri/package.json`。`.github/workflows/desktop-shell.yml` 让 macOS、Windows、Ubuntu 各自在原生架构的 runner 上完成上述构建，并把五个平台包上传为 artifact；分支 push 只跑构建，不会发布。

## 发布

版本号在 `Tauri/shell-version.json`。壳改动后抬版本、提交，再推一个与版本同名的 tag：

```sh
git tag shell-v0.1.0
git push origin shell-v0.1.0
```

tag 触发 CI 编出五个平台包，并用 [npm 可信发布](https://docs.npmjs.com/trusted-publishers)一起发到 npm。CI 不存任何 npm 令牌：npm 认的是本仓库与 `desktop-shell.yml` 这条流水线，由 GitHub 现场签一张短效身份凭证。发之前会核对 tag、版本文件与五个包内版本一致，发完核对注册表里五个版本都到位。分支 push 和手动触发只编不发。

每个平台包要在 npm 设置页登记一次可信发布关系（GitHub 仓库 `WNLUO/dsh-mobile-shell`、工作流文件 `desktop-shell.yml`），登记一次即固定，之后不用再动。设置页：

- [dsh-mobile-shell-darwin-arm64](https://www.npmjs.com/package/dsh-mobile-shell-darwin-arm64/access)
- [dsh-mobile-shell-darwin-x64](https://www.npmjs.com/package/dsh-mobile-shell-darwin-x64/access)
- [dsh-mobile-shell-linux-x64](https://www.npmjs.com/package/dsh-mobile-shell-linux-x64/access)
- [dsh-mobile-shell-linux-arm64](https://www.npmjs.com/package/dsh-mobile-shell-linux-arm64/access)
- [dsh-mobile-shell-win32-x64](https://www.npmjs.com/package/dsh-mobile-shell-win32-x64/access)

npm 不给尚不存在的包提供设置页，所以每个包的第一个版本要先由维护者本机手动发布一次；包存在之后才登记信任关系，再往后就全由 tag 自动发布。

本地手动发版（五个平台包目录齐备时）：

```sh
node tools/publish-desktop-shell.mjs --dry-run   # 只看要发什么
node tools/publish-desktop-shell.mjs             # 发五个平台包到 npm
node tools/publish-desktop-shell.mjs --wire      # 打印应写进插件 optionalDependencies 的五条依赖
```

顺序不能反：平台包先上 npm，插件才把版本号登记进 `optionalDependencies` 并更新锁文件，然后才发主插件包，这样主包带的是已经存在于 npm 上的依赖。插件清单在插件仓库维护，本仓库的发布脚本只打印需要写入的五个包名与版本号。

## 目录

- `Tauri/`：Tauri 2 源码、配置、图标与 `shell-version.json`
- `tools/build-desktop-shell.mjs`：把裸产物打成按平台的 npm 包
- `tools/publish-desktop-shell.mjs`：发布五个平台包
- `.github/workflows/desktop-shell.yml`：五平台构建与发布流水线

## 许可

MIT，见 [LICENSE](LICENSE)。运行时链接的 wry、tao 与各平台系统 WebView 的许可与版权声明以各自项目为准。
