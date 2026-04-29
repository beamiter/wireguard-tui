# 快速开始指南

## 5 分钟快速上手

### 第 1 步：编译

```bash
cd wireguard-tui
cargo build --release
```

### 第 2 步：配置凭证（自动生成）

**✨ 新功能：配置文件自动生成！**

首次运行时，应用会自动创建配置模板在 `~/.config/wireguard-tui/config.toml`

**只需编辑两个字段：**

```bash
nano ~/.config/wireguard-tui/config.toml
```

更新这两行：
```toml
username = "your-actual-username"  # 你的 StrongVPN 用户名
password = "your-actual-password"  # 你的 StrongVPN 密码
```

💡 应用会提示你配置文件的确切位置！

### 第 3 步：运行应用

```bash
./target/release/wireguard-tui
```

或者直接运行（使用 debug 版本）：

```bash
cargo run
```

## 基本操作

### 首次使用流程（浏览器 + 导入）

```
1. 启动应用
   ↓
2. 按 'o' 在浏览器打开下载页面
   ↓
3. 在浏览器登录并下载服务器配置（*.conf）
   ↓
4. 回到 TUI，按 'i' 导入配置
   ↓
5. 用 ↑↓ 选择配置，按 Enter 导入（或按 'a' 全部导入）
   ↓
6. 按方向键 ↑↓ 选择服务器
   ↓
7. 按 Enter 连接
   ↓
8. 看到 ● 符号表示已连接
   ↓
9. 按 's' 查看连接详情和 IP 地址
```

### 常用快捷键

| 快捷键 | 功能 | 何时用 |
|---------|------|--------|
| `↑` / `↓` | 上下导航 | 浏览服务器列表 |
| `Enter` | 连接/断开 | 选中服务器后连接 |
| `r` | 刷新/下载配置 | 首次使用或需要更新服务器 |
| `s` | 查看状态 | 检查连接信息和 IP |
| `d` | 删除配置 | 删除不需要的服务器 |
| `q` | 退出应用 | 离开应用 |

## 图示指南

### 主界面解读

```
🔒 WireGuard VPN Manager          ← 应用标题
✓ Connected: server-us-001        ← 连接状态

Available Servers:                ← 服务器列表
● server-us-001                   ← ● 表示已连接
○ server-us-002                   ← ○ 表示可用
○ server-eu-001
○ server-asia-001

Ready                             ← 消息显示
↑↓: Navigate | Enter: Connect | r: Download | d: Delete | s: Status | q: Quit  ← 帮助信息
```

### 状态详情界面

按 `s` 后显示：

```
┌─ Connection Details ─────────────────────────────────────────────┐
│                                                                   │
│ Interface: server-us-001                                          │
│ Status: Connected ✓                                               │
│                                                                   │
│ Endpoint: 108.171.121.213:58493                                   │
│ Allowed IPs: 0.0.0.0/0                                            │
│                                                                   │
│ Listening Port: 49866                                             │
│ Latest Handshake: 2 minutes, 30 seconds ago                       │
│                                                                   │
│ Received: 5.43 MiB                                                │
│ Sent: 2.10 MiB                                                    │
│                                                                   │
│ Public Key: rvxiv4p6gzewnqgjcn28jn6+naeewewrw...                 │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

## 故障排除

### 问题：显示 "No servers available"

**解决方案：**
1. 按 `r` 下载配置文件
2. 等待下载完成
3. 如果下载失败，检查：
   - 网络连接
   - 凭证是否正确（用户名和密码）
   - StrongVPN 账户是否有效

### 问题：连接失败

**解决方案：**
1. 尝试连接不同的服务器
2. 重启应用
3. 检查系统是否已安装 WireGuard：
   ```bash
   sudo wg --version
   ```

### 问题：提示权限不足

**解决方案：**
```bash
# 应用会提示输入 sudo 密码
# 按照提示输入密码即可

# 或者预先配置 sudo 无需密码（不推荐）
```

### 问题：无法下载配置

**解决方案：**
1. 访问 https://tools.strongvpn.asia/share/strong-wg/strong-wg.html
2. 验证凭证是否正确
3. 检查网络连接：
   ```bash
   curl -I https://tools.strongvpn.asia/share/strong-wg/strong-wg.html
   ```

## 验证连接

### 方法 1：通过应用

按 `s` 查看状态界面，检查是否有流量流动：
- `Received` 和 `Sent` 应该都在增加
- `Latest Handshake` 应该是最近的时间

### 方法 2：通过网站

访问 https://wg.strongtech.org/ipcheck 查看当前 IP：
- 应该显示 VPN 的 IP
- 而不是你的实际 IP

### 方法 3：通过命令行

```bash
# 查看 WireGuard 状态
sudo wg show <interface-name>

# 查看连接的 IP
curl -s https://api.ipify.org

# 查看完整信息
ip addr show | grep wg
```

## 配置文件位置

```
~/.config/wireguard-tui/config.toml    ← 应用配置
/etc/wireguard/                        ← WireGuard 配置文件目录
```

## 日志和调试

启用调试输出：

```bash
RUST_LOG=debug ./target/release/wireguard-tui
```

检查系统日志：

```bash
# Ubuntu/Debian
sudo journalctl -u wireguard -f

# 或查看 syslog
sudo tail -f /var/log/syslog | grep wireguard
```

## 高级用法

### 编辑配置文件

```bash
nano ~/.config/wireguard-tui/config.toml
```

选项：
- `username`: StrongVPN 用户名（需要更新）
- `password`: StrongVPN 密码（需要更新）
- `auto_download`: 启动时自动下载（true/false）
- `last_server`: 上次连接的服务器（自动保存）

### 手动管理 WireGuard

```bash
# 列出所有配置
ls /etc/wireguard/

# 手动连接
sudo wg-quick up <config-name>

# 手动断开
sudo wg-quick down <config-name>

# 查看状态
sudo wg show <config-name>
```

## 常见功能流程

### 流程 1：切换服务器

```
当前连接 ← 按 ↓ 选择新服务器 ← 按 Enter → 自动断开旧连接 → 连接新服务器
```

### 流程 2：更新配置列表

```
按 'r' → 输入凭证 → 下载所有配置 → 列表更新 → 选择新服务器
```

### 流程 3：删除配置

```
按 ↑↓ 选择 → 按 'd' → 自动断开（如果已连接） → 删除文件
```

## 性能和系统资源

- **内存占用**: ~10-20 MB
- **CPU 占用**: 空闲时 < 1%
- **磁盘占用**: 配置文件 ~1 MB

## 安全性注意

⚠️ **重要提示：**

1. **不要将凭证共享给他人**
   - 凭证保存在本地配置文件中
   - 不要上传到 GitHub 或其他公共位置

2. **定期更新密码**
   - 如果怀疑泄露，立即更改 StrongVPN 密码

3. **信任网络**
   - 在不信任的网络上使用 VPN
   - 不要禁用 VPN 进行敏感操作

## 获取帮助

- 📖 完整文档：查看 README.md
- 🐛 报告问题：提交 Issue
- 💬 提问讨论：使用 Discussion
- 🔗 StrongVPN 支持：https://support.strongtech.org/hc/zh-cn/
