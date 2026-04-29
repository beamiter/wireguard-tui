# WireGuard TUI Manager

一个用 Rust 编写的全功能 WireGuard VPN 管理器 TUI 应用程序。

## 功能特性

✨ **核心功能：**
- 🔒 自动安装 WireGuard
- 📥 从 StrongVPN 下载 WireGuard 配置文件
- 🔌 一键连接/断开 VPN
- 📊 实时连接状态监控
- 📈 流量统计信息
- 🗂️ 配置文件管理
- 🖥️ 交互式 TUI 界面

## 安装

### 前置要求
- Rust 1.70+ ([安装 Rust](https://rustup.rs/))
- Linux (Ubuntu, Debian, Fedora, Arch, openSUSE 等)
- sudo 权限（用于 WireGuard 操作）

### 编译

```bash
cd wireguard-tui
cargo build --release
```

编译后的可执行文件在 `target/release/wireguard-tui`

## 配置

### ✨ 自动配置生成（新功能）

**首次运行时，配置文件会自动生成！**

```bash
./target/release/wireguard-tui
```

应用会：
1. ✅ **自动创建**配置模板 `~/.config/wireguard-tui/config.toml`
2. ✅ 检查 WireGuard 是否安装
3. ✅ 如果未安装，自动安装 WireGuard 工具
4. ⚠️ 提示你编辑配置文件

### 编辑配置

```bash
# 打开配置文件
nano ~/.config/wireguard-tui/config.toml
```

**只需更新这两行：**

```toml
username = "your-actual-username"  # 改为你的 StrongVPN 用户名
password = "your-actual-password"  # 改为你的 StrongVPN 密码
```

配置文件包含详细注释说明，位置：`~/.config/wireguard-tui/config.toml`

💡 **提示：** 应用会自动检测凭证是否为模板默认值，并在界面提示你更新。

📖 **详细说明：** 查看 [AUTO_CONFIG.md](AUTO_CONFIG.md) 了解自动配置功能的完整文档

## 使用方法

### 导航

| 按键 | 功能 |
|------|------|
| `↑` `↓` | 上下移动选择 |
| `Enter` | 连接/断开选中的服务器 |
| `o` | ✨ 在浏览器中打开下载页面 |
| `i` | ✨ 导入下载的配置文件 |
| `d` | 删除选中的配置文件 |
| `s` | 查看连接状态详情 |
| `q` 或 `Ctrl+C` | 退出应用 |

### 工作流程

#### 1. 下载配置文件（浏览器 + 导入）

**步骤 1：** 按 `o` 在浏览器打开下载页面
- 自动打开 https://tools.strongvpn.asia/share/strong-wg/strong-wg.html

**步骤 2：** 在浏览器中下载
- 输入你的 StrongVPN 凭证登录
- 选择需要的服务器
- 下载 `.conf` 文件到 `~/Downloads/`

**步骤 3：** 按 `i` 导入配置
- 应用扫描 `~/Downloads/` 目录
- 显示找到的配置文件列表
- 用 `↑↓` 选择，`Enter` 导入
- 或按 `a` 导入全部

💡 **提示：** 详细说明请查看 [BROWSER_IMPORT.md](BROWSER_IMPORT.md)

#### 2. 连接 VPN
- 使用 `↑↓` 箭头键选择想要的服务器
- 按 `Enter` 连接
- 连接成功后，服务器前会显示 `●` 符号

#### 3. 查看连接状态
- 按 `s` 键查看详细的连接信息
- 包括：IP 地址、流量统计、端点、握手信息等

#### 4. 断开 VPN
- 选中已连接的服务器
- 按 `Enter` 断开连接

#### 5. 删除配置
- 选中配置文件
- 按 `d` 删除（会自动断开连接）

## 界面说明

### 主界面
```
🔒 WireGuard VPN Manager

Available Servers:
● server-1
○ server-2
○ server-3

Status | ↑↓: Navigate | Enter: Connect/Disconnect | r: Download | d: Delete | s: Status | q: Quit
```

- **●** = 当前已连接的服务器
- **○** = 可用的服务器

### 连接状态界面
显示以下信息：
- 接口名称
- 连接状态
- 端点地址
- 允许的 IP
- 监听端口
- 最后握手时间
- 接收/发送流量
- 公钥

## 常见问题

### Q: 权限被拒绝？
A: WireGuard 命令需要 sudo 权限。应用会自动提示输入密码。确保你的账户在 sudoers 文件中。

### Q: 无法连接到服务器？
A: 
1. 检查网络连接
2. 尝试连接到不同的服务器（不同时间服务器连接性不同）
3. 查看详细的错误信息

### Q: 如何验证 VPN 连接？
A: 
1. 按 `s` 查看连接状态
2. 查看 "Received" 和 "Sent" 的流量数据
3. 访问 https://wg.strongtech.org/ipcheck 查看你的新 IP

### Q: 配置文件在哪里？
A: `/etc/wireguard/` 目录

## 架构

```
wireguard-tui/
├── src/
│   ├── main.rs       # 主入口、事件循环
│   ├── app.rs        # 应用状态管理
│   ├── ui.rs         # TUI 渲染
│   ├── vpn.rs        # VPN 操作
│   ├── download.rs   # 配置下载
│   ├── config.rs     # 配置管理
│   └── commands.rs   # 系统命令执行
└── Cargo.toml
```

## 技术栈

- **ratatui** - TUI 框架
- **tokio** - 异步运行时
- **reqwest** - HTTP 客户端
- **crossterm** - 终端操作
- **serde** - 序列化

## 故障排除

### 调试日志
如果遇到问题，可以使用 RUST_LOG 环境变量启用调试日志：

```bash
RUST_LOG=debug ./target/release/wireguard-tui
```

### 常见错误

1. **"WireGuard not installed"**
   - 应用会自动尝试安装
   - 如果自动安装失败，请手动安装

2. **"Failed to download configs"**
   - 检查凭证是否正确
   - 检查网络连接
   - 访问下载页面验证账户

3. **"Permission denied"**
   - 确保你有 sudo 权限
   - 某些系统可能需要特殊配置

## 支持

- 官方 WireGuard 文档：https://www.wireguard.com/
- StrongVPN 支持：https://support.strongtech.org/hc/zh-cn/
- 本项目问题反馈

## 许可证

MIT License
