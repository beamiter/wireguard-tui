# 运行指南

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

### 步骤 3：使用 sudo 运行应用

由于 WireGuard 配置在 `/etc/wireguard/`，应用需要 sudo 权限：

```bash
sudo ./target/debug/wireguard-tui
```

或release 版本：

```bash
sudo ./target/release/wireguard-tui
```

### 步骤 4：测试导入功能

1. 启动应用（用 sudo）
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
sudo ./target/debug/wireguard-tui 2>&1 | tee debug.log
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

### Q: 为什么需要 sudo？

**A:** 因为：
1. `/etc/wireguard/` 目录只有 root 可读写
2. `wg-quick` 命令需要 root 权限
3. 导入配置时需要复制文件到 `/etc/wireguard/`

### Q: 能不用 sudo 运行吗？

**A:** 可以查看下载信息（按 `o`），但：
- 无法导入配置
- 无法连接 VPN
- 无法查看已有配置

**解决方案：** 始终用 `sudo` 运行：
```bash
sudo ./target/release/wireguard-tui
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

### 运行（重要：使用 sudo）
```bash
sudo ./target/release/wireguard-tui
```

### 操作流程

```
1. sudo ./target/release/wireguard-tui
2. 按 'o' 查看下载信息
3. 手动到浏览器下载配置
4. 按 'i' 导入（会显示文件列表）
5. Space 勾选/取消勾选
6. Enter 导入
7. ↑↓ 选择服务器
8. Enter 连接
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

# 3. 运行应用
sudo ./target/debug/wireguard-tui 2>&1 | tee test.log &

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
   sudo ./target/debug/wireguard-tui 2>&1 | tee full-debug.log
   # 按 'i'，然后 'q' 退出
   cat full-debug.log
   ```

4. **测试脚本输出：**
   ```bash
   ./debug_import.sh
   ```

---

**记住：** 必须用 `sudo` 运行才能完整使用所有功能！
