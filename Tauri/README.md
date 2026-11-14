# 远程电脑桌面显示壳

这个 Tauri 2 桌面端不是独立应用，也不做安装器、不注册自定义协议、不做单实例：它是 `dsh-mobile-plugin` 随包按平台分发的显示组件，只用来显示「远程电脑」的一次性网页入口。

插件在 B 电脑的控制台点「在桌面端打开」时，先向本机 `POST /api/relay/remote-computers/open-shell` 拿到 `{ ok, url, expiresAt }`，再 spawn 平台包里的可执行文件：

```sh
dsh-mobile-shell --remote-url=<一次性地址> --device-name=<设备名>
```

一次点击启动一个进程、开一个窗口，窗口标题带设备名；多台设备可以同时多开，互不干扰。机器上没装对应平台包时接口返回 `SHELL_UNAVAILABLE`，插件自动回退到原来的系统浏览器路径，远程电脑功能照常可用。

## 分发

平台包 `dsh-mobile-shell-<os>-<cpu>` 由 `tools/build-desktop-shell.mjs` 打包、`tools/publish-desktop-shell.mjs` 发布，走 npm 的可选依赖链路，不做运行时下载。版本取 `Tauri/shell-version.json`，独立于插件版本：壳没改就不抬版本，插件更新不会重下平台包。

可执行文件统一名 `dsh-mobile-shell`（`tauri.conf.json` 的 `mainBinaryName`），平台包内的相对路径：

- `darwin-arm64` / `darwin-x64`：`dsh-mobile-shell.app/Contents/MacOS/dsh-mobile-shell`
- `linux-x64` / `linux-arm64`：`dsh-mobile-shell`
- `win32-x64`：`dsh-mobile-shell.exe`

Tauri 依赖各平台的系统 WebView，不能在一台机器上交叉编译五个平台。`.github/workflows/desktop-shell.yml` 让 macOS / Windows / Ubuntu 各自在原生架构的 runner 上只构建裸应用产物，再由 `tools/build-desktop-shell.mjs` 从 `Tauri/src-tauri/target/release/` 收产物、打成 `build/desktop-shell/<平台>/` 下的平台包。
