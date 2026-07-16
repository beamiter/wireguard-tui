# 运行指南

> [!WARNING]
> 这是已归档的早期排障记录，不适用于 v0.4，也不应捕获或分享完整终端日志。请只使用当前 [README.md](README.md) 与 [SECURITY.md](SECURITY.md)。

## 问题诊断

如果你遇到 "No .conf files found" 错误，但文件确实存在：

### 步骤 1：验证文件存在

```bash
ls -la ~/Downloads/*.conf
```

**应该看到：**
```
-rw-rw-r-- 1 mm mm 244  4月 29 23:26 /home/mm/Downloads/str-dub303.conf
```

### 步骤 2：运行调试脚本

```bash
./debug_import.sh
```

这会测试扫描逻辑是否正常。

### 步骤 3：预授权后以普通用户运行应用

WireGuard 操作需要有限的提权权限，但 TUI 本身应保持为普通用户进程：

```bash
sudo -v
./target/debug/wireguard-tui
```

或release 版本：

```bash
sudo -v
./target/release/wireguard-tui
```

### 步骤 4：测试导入功能

1. 先执行 `sudo -v`，随后以普通用户启动应用
2. 按 `i` 键
3. 应该看到文件列表

**预期界面：**
```
┌─ Import WireGuard Configurations ─────────────┐
│ Found 1 config(s) - 1 selected                 │
│                                                │
│ ▶ [✓] str-dub303.conf (244 bytes, 5 min ago)  │
│                                                │
└────────────────────────────────────────────────┘

↑↓: Navigate | Space: Check/Uncheck | Enter: Import
```

## 调试输出

运行时会在 stderr 输出调试信息：

```bash
sudo -v
./target/debug/wireguard-tui 2>&1 | tee debug.log
```

然后按 `i`，你会看到：

```
DEBUG: Scanning directory: "/home/mm/Downloads"
DEBUG: Directory exists: true
DEBUG: Found file: "/home/mm/Downloads/str-dub303.conf"
DEBUG: Extension: "conf"
DEBUG: Adding config: "/home/mm/Downloads/str-dub303.conf"
DEBUG: Total configs found: 1
DEBUG: scan_downloads returned 1 files
```

## 常见问题

### Q: 为什么需要 sudo 凭据？

**A:** 因为：
1. `/etc/wireguard/` 目录只有 root 可读写
2. `wg-quick` 命令需要 root 权限
3. 导入配置时需要复制文件到 `/etc/wireguard/`

### Q: 能不做 sudo 预授权吗？

**A:** 可以启动和浏览不需要提权的页面，但需要访问 `/etc/wireguard` 或管理接口的操作可能因非交互式 sudo 无可用凭据而失败。

**解决方案：** 先刷新 sudo 缓存，再以普通用户运行；不要把 sudo 直接放在 TUI 命令前：
```bash
sudo -v
./target/release/wireguard-tui
```

### Q: 扫描逻辑正确吗？

**A:** 是的！运行测试验证：
```bash
./debug_import.sh
```

输出会显示找到的文件。

## 正确的使用方式

### 编译
```bash
cargo build --release
```

### 运行（重要：仅预授权，TUI 使用普通用户）
```bash
sudo -v
./target/release/wireguard-tui
```

### 操作流程

```
1. `sudo -v`
2. `./target/release/wireguard-tui`
3. 按 'o' 查看下载信息
4. 手动到浏览器下载配置
5. 按 'i' 导入（会显示文件列表）
6. Space 勾选/取消勾选
7. Enter 导入
8. ↑↓ 选择服务器
9. Enter 连接
```

## 验证安装

### 检查 WireGuard

```bash
which wg-quick
wg-quick --version
```

### 检查目录权限

```bash
ls -ld /etc/wireguard
```

应该显示：
```
drwx------ 2 root root 4096 ... /etc/wireguard
```

### 检查配置

```bash
sudo ls -la /etc/wireguard/
```

## 完整测试流程

```bash
# 1. 清理环境
rm -f ~/Downloads/test.conf

# 2. 创建测试配置
echo "[Interface]" > ~/Downloads/test.conf
echo "PrivateKey = test" >> ~/Downloads/test.conf

# 3. 预授权后以普通用户运行应用
sudo -v
./target/debug/wireguard-tui 2>&1 | tee test.log &

# 4. 按 'i' 测试导入

# 5. 检查 test.log
grep "DEBUG" test.log
```

## 如果还是不行

请提供以下信息：

1. **系统信息：**
   ```bash
   uname -a
   ```

2. **文件列表：**
   ```bash
   ls -la ~/Downloads/*.conf
   ```

3. **调试输出：**
   ```bash
   sudo -v
   ./target/debug/wireguard-tui 2>&1 | tee full-debug.log
   # 按 'i'，然后 'q' 退出
   cat full-debug.log
   ```

4. **测试脚本输出：**
   ```bash
   ./debug_import.sh
   ```

---

**记住：** 使用 `sudo -v` 只为后续受限操作预授权；TUI 始终由普通用户启动。
