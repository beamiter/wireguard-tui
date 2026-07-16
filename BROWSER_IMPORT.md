# 浏览器集成 + 导入功能

> [!WARNING]
> 这是已归档的 v0.3 设计记录，不适用于 v0.4。不要按本文保存账号密码、手工 `sudo cp` 或绕过导入校验；请只使用当前 [README.md](README.md)。

## 概述

WireGuard TUI 现在支持**浏览器集成**和**配置文件导入**！不再需要自动下载，而是：
1. 在浏览器中手动下载配置
2. 使用 TUI 导入到系统

## 🎯 工作流程

### 完整流程（3 步）

```
1. 按 'o' 打开浏览器 → 下载页面
2. 在浏览器登录并下载配置文件
3. 按 'i' 导入配置到 WireGuard
```

## 📖 详细步骤

### 步骤 1：查看下载信息

在 TUI 主界面按 `o` 键：

```
┌─ Download Instructions ────────────────────────┐
│                                                │
│ Step 1: Open this URL in your browser:        │
│                                                │
│   https://tools.strongvpn.asia/share/...      │
│                                                │
│ ──────────────────────────────────────────────│
│                                                │
│ Step 2: Login with your credentials:          │
│                                                │
│   Username: your-vpn-username                            │
│   Password: your-vpn-password                         │
│                                                │
│ ──────────────────────────────────────────────│
│                                                │
│ Step 3: Download server configs                │
│   • Select servers you want                    │
│   • Download to ~/Downloads/                   │
│   • Files format: str-*.conf                   │
│                                                │
│ ──────────────────────────────────────────────│
│                                                │
│ Step 4: Press 'i' to import                    │
│                                                │
│ Press Esc to return                            │
└────────────────────────────────────────────────┘
```

**你需要做：**
- 复制 URL 到浏览器
- 复制用户名和密码登录

### 步骤 2：在浏览器中下载

**在浏览器中：**

1. 输入凭证登录
   - Username: `your-vpn-username`（你的实际用户名）
   - Password: `your-vpn-password`（你的实际密码）

2. 选择服务器
   - 浏览可用的服务器列表
   - 点击想要的服务器

3. 下载配置文件
   - 文件格式：`str-zrh302.conf`
   - 保存到：`~/Downloads/`

**提示：** 你可以下载多个配置文件！

### 步骤 3：导入配置

回到 TUI，按 `i` 键：

```
┌─ Import WireGuard Configurations ─────────────┐
│                                                │
│ Found 3 config(s) in ~/Downloads               │
│                                                │
│ ▶ str-us-001.conf (2048 bytes, 2 minutes ago) │
│   str-eu-002.conf (2056 bytes, 3 minutes ago) │
│   str-asia-101.conf (2032 bytes, 5 minutes ago)│
│                                                │
│ ↑↓: Select | Enter: Import | a: Import all    │
└────────────────────────────────────────────────┘
```

**操作：**
- `↑↓` - 选择配置文件
- `Enter` - 导入选中的文件
- `a` - 导入全部文件
- `Esc` - 取消并返回

### 导入成功

```
┌────────────────────────────────────────────────┐
│ 🔒 WireGuard VPN Manager                       │
│ ✗ Not Connected                                │
│                                                │
│ Available Servers:                             │
│ ○ str-us-001                                   │
│ ○ str-eu-002                                   │
│ ○ str-asia-101                                 │
│                                                │
│ ✓ Imported str-us-001.conf                     │
└────────────────────────────────────────────────┘
```

现在可以用 `↑↓` 选择并 `Enter` 连接！

## 🎮 快捷键

### 主界面

| 键 | 功能 |
|----|------|
| `o` | 在浏览器中打开下载页面 |
| `i` | 导入 Downloads 中的配置文件 |
| `↑` `↓` | 导航服务器列表 |
| `Enter` | 连接/断开 VPN |
| `d` | 删除配置文件 |
| `s` | 查看连接状态 |
| `q` | 退出 |

### 导入界面

| 键 | 功能 |
|----|------|
| `↑` `↓` | 选择配置文件 |
| `Enter` | 导入选中的文件 |
| `a` | 导入全部文件 |
| `Esc` | 返回主界面 |

## 🔍 智能功能

### 自动扫描

按 `i` 时，应用会：
- 扫描 `~/Downloads/` 目录
- 查找 `str-*.conf` 格式的文件
- 按修改时间排序（最新在前）
- 显示文件大小和下载时间

### 文件识别

只导入符合格式的文件：
- ✅ `str-zrh302.conf` - 识别
- ✅ `str-us-001.conf` - 识别
- ❌ `config.conf` - 忽略（不是 str- 开头）
- ❌ `wireguard.txt` - 忽略（不是 .conf）

### 安全导入

- 使用 `sudo` 复制到 `/etc/wireguard/`
- 保留原始文件在 `~/Downloads/`
- 检测重复文件
- 验证权限

## 📂 文件位置

### 下载位置
```
~/Downloads/str-*.conf
```

### 导入后位置
```
/etc/wireguard/str-*.conf
```

### 配置格式示例

```
[Interface]
PrivateKey = ...
Address = 10.0.0.2/24
DNS = 8.8.8.8

[Peer]
PublicKey = ...
Endpoint = 108.171.121.213:58493
AllowedIPs = 0.0.0.0/0
PersistentKeepalive = 25
```

## 🛠️ 故障排除

### 问题：按 'o' 没反应

**原因：** 系统没有 `xdg-open` 命令

**解决：**
```bash
# 手动打开浏览器访问
firefox https://tools.strongvpn.asia/share/strong-wg/strong-wg.html

# 或
google-chrome https://tools.strongvpn.asia/share/strong-wg/strong-wg.html
```

### 问题：找不到配置文件

**检查下载位置：**
```bash
ls ~/Downloads/str-*.conf
```

**如果在其他位置，移动到 Downloads：**
```bash
mv /path/to/str-*.conf ~/Downloads/
```

### 问题：导入失败

**常见原因：**
1. **权限不足** - 需要 sudo 权限
2. **文件损坏** - 重新下载
3. **文件格式错误** - 检查是否是正确的 .conf 文件

**手动导入：**
```bash
sudo cp ~/Downloads/str-zrh302.conf /etc/wireguard/
```

### 问题：导入后看不到服务器

**刷新列表：**
- 退出并重新打开 TUI
- 或按 `Esc` 返回主界面

## 💡 使用技巧

### 技巧 1：批量下载

在浏览器中一次下载多个服务器配置，然后用 `a` 键批量导入。

### 技巧 2：测试服务器

先下载几个不同地区的服务器配置，测试后保留最快的。

### 技巧 3：定期更新

每月重新下载配置，删除旧的：
```bash
# 清理旧配置
sudo rm /etc/wireguard/str-*.conf

# 重新下载并导入
```

### 技巧 4：备份配置

保存常用的配置文件：
```bash
mkdir -p ~/wireguard-backup
cp ~/Downloads/str-*.conf ~/wireguard-backup/
```

## 🎯 完整示例

### 首次设置

```bash
# 1. 启动 TUI
./target/release/wireguard-tui

# 2. 在 TUI 中按 'o'
# 浏览器自动打开下载页面

# 3. 在浏览器登录
# Username: your-vpn-username
# Password: your-vpn-password

# 4. 下载几个服务器配置
# str-us-001.conf
# str-eu-002.conf  
# str-asia-101.conf

# 5. 回到 TUI，按 'i'
# 6. 按 'a' 导入全部
# 7. 用 ↑↓ 选择服务器
# 8. 按 Enter 连接

✓ 完成！
```

## 🔐 安全说明

### 凭证安全
- 浏览器会记住密码（如果允许）
- 不要在公共电脑上保存凭证
- 定期更改密码

### 文件安全
- 配置文件包含私钥
- 不要分享给他人
- 不要上传到公共位置

### 权限说明
```bash
# 导入需要 sudo 权限
# 会提示输入密码

sudo cp ~/Downloads/str-*.conf /etc/wireguard/
```

## 📊 技术细节

### 浏览器打开

**Linux:**
```rust
Command::new("xdg-open")
    .arg(url)
    .spawn()
```

**macOS:**
```rust
Command::new("open")
    .arg(url)
    .spawn()
```

**Windows:**
```rust
Command::new("cmd")
    .args(&["/C", "start", url])
    .spawn()
```

### 文件扫描

```rust
// 扫描 Downloads 目录
for entry in fs::read_dir(&downloads_dir)? {
    let path = entry.path();
    
    // 检查格式：str-*.conf
    if path.starts_with("str-") && path.ends_with(".conf") {
        configs.push(path);
    }
}

// 按时间排序
configs.sort_by(|a, b| {
    b.modified().cmp(&a.modified())
});
```

### 导入操作

```rust
// 使用 sudo 复制
Command::new("sudo")
    .arg("cp")
    .arg(source_path)
    .arg("/etc/wireguard/")
    .status()
```

## 🆚 对比：自动下载 vs 手动导入

| 功能 | 自动下载 | 浏览器 + 导入 |
|------|----------|---------------|
| 需要编程 | 是 | 否 |
| 灵活性 | 低 | 高 |
| 服务器选择 | 全部 | 自由选择 |
| 登录方式 | HTTP 基础认证 | 浏览器表单 |
| 验证码支持 | 否 | 是 |
| 2FA 支持 | 否 | 是 |
| 用户控制 | 低 | 高 |
| 错误处理 | 复杂 | 简单 |

**结论：** 手动方式更灵活，支持各种认证方式！

## 🔄 迁移指南

### 从自动下载迁移

如果你之前使用自动下载功能：

1. **删除旧快捷键习惯**
   - 不再使用 `r` 键下载
   - 改用 `o` + `i` 组合

2. **清理旧配置**
   ```bash
   sudo rm /etc/wireguard/str-*.conf
   ```

3. **重新导入**
   - 按 `o` 打开浏览器
   - 下载需要的配置
   - 按 `i` 导入

## 📝 常见问题

### Q: 为什么不自动下载？

A: 因为下载页面需要：
- 用户登录
- 可能有验证码
- 可能有 2FA
- 需要手动选择服务器

手动方式更可靠！

### Q: 可以直接拖放文件吗？

A: 目前不支持，但你可以：
1. 手动复制到 Downloads
2. 按 `i` 导入

### Q: 导入后文件会删除吗？

A: 不会，原文件保留在 Downloads。你可以手动删除：
```bash
rm ~/Downloads/str-*.conf
```

### Q: 支持其他 VPN 提供商吗？

A: 目前只识别 `str-*.conf` 格式的 StrongVPN 配置。

## 🎓 最佳实践

1. **定期更新配置** - 每月重新下载
2. **测试多个服务器** - 找出最快的
3. **备份常用配置** - 保存到安全位置
4. **清理旧文件** - 删除不用的配置
5. **验证连接** - 导入后测试连接

---

**版本：** v0.3.0（浏览器集成）  
**更新日期：** 2026-04-29  
**状态：** ✅ 已实现并测试
