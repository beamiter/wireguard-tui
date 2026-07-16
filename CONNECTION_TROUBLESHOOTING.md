# 连接故障排除

> 历史兼容说明：v0.4 不允许以 root 身份运行整个 TUI。需要排障时先执行 `sudo -v`，然后以当前普通用户启动应用；下文仅在单独的系统诊断命令确实需要时使用 sudo。

## 问题：Connection failed

你看到的错误：
```
Connection failed: Failed to connect: [#] ip link add str-dub303 type wireguard
 [#] wg setconf str-dub303 /dev/fd/63
```

这说明：
- ✅ 导入成功了（能找到并尝试连接配置）
- ❌ WireGuard 命令执行时出错

## 🔍 可能的原因

### 1. WireGuard 内核模块未加载

**检查：**
```bash
lsmod | grep wireguard
```

**如果没有输出，加载模块：**
```bash
sudo modprobe wireguard
```

**如果提示模块不存在：**
```bash
# 检查内核版本
uname -r

# 安装 WireGuard 模块
sudo apt install wireguard-dkms  # Ubuntu/Debian
```

### 2. 配置文件格式错误

**检查配置：**
```bash
sudo sed -E 's/^([[:space:]]*(PrivateKey|PresharedKey)[[:space:]]*=[[:space:]]*).*/\1<HIDDEN>/I' -- /etc/wireguard/str-dub303.conf
```

**正确的格式应该是：**
```ini
[Interface]
PrivateKey = <your-private-key>
Address = 10.0.0.2/24
DNS = 8.8.8.8

[Peer]
PublicKey = <server-public-key>
Endpoint = <server-ip>:<port>
AllowedIPs = 0.0.0.0/0
PersistentKeepalive = 25
```

### 3. 网络接口已存在

**检查：**
```bash
ip link show | grep str-dub303
```

**如果接口已存在，先删除：**
```bash
sudo wg-quick down str-dub303
# 或
sudo ip link delete str-dub303
```

### 4. 权限问题

**刷新 sudo 凭据后，以普通用户运行 TUI：**
```bash
sudo -v
./target/release/wireguard-tui
```

## 🧪 诊断步骤

### 步骤 1：运行测试脚本

```bash
./test_connect.sh
```

这会自动检查：
- WireGuard 安装
- 内核模块
- 配置文件
- 尝试手动连接

### 步骤 2：手动测试连接

```bash
# 手动连接
sudo wg-quick up str-dub303

# 查看详细输出
echo $?  # 检查退出码
```

**成功的输出应该类似：**
```
[#] ip link add str-dub303 type wireguard
[#] wg setconf str-dub303 /dev/fd/63
[#] ip -4 address add 10.0.0.2/24 dev str-dub303
[#] ip link set mtu 1420 up dev str-dub303
[#] resolvconf -a tun.str-dub303 -m 0 -x
[#] ip -4 route add 0.0.0.0/0 dev str-dub303 table 51820
[#] ip -4 rule add not fwmark 51820 table 51820
```

### 步骤 3：检查详细错误

```bash
# 在本机终端观察错误；不要把原始输出写入日志或直接分享，
# 无效密钥的底层诊断可能包含原始密钥值。
sudo wg-quick up str-dub303
```

### 步骤 4：检查系统日志

```bash
# 查看 dmesg
sudo dmesg | tail -20

# 查看 syslog
sudo tail -50 /var/log/syslog | grep wireguard
```

## 🔧 常见解决方案

### 解决方案 1：加载 WireGuard 模块

```bash
# 加载模块
sudo modprobe wireguard

# 验证
lsmod | grep wireguard
```

**输出应该类似：**
```
wireguard             xxx  0
```

### 解决方案 2：重新下载配置

配置文件可能损坏或不完整：

1. 删除现有配置：
   ```bash
   sudo rm /etc/wireguard/str-dub303.conf
   ```

2. 在 TUI 中按 `o` 查看下载信息

3. 重新在浏览器下载配置

4. 按 `i` 重新导入

### 解决方案 3：检查网络

```bash
# 测试网络连接
ping -c 3 8.8.8.8

# 检查 DNS
nslookup google.com
```

### 解决方案 4：清理旧连接

```bash
# 列出所有 WireGuard 接口
ip link show | grep wg

# 删除旧接口
sudo ip link delete <interface-name>

# 或使用 wg-quick
sudo wg-quick down <config-name>
```

## 📋 完整诊断流程

```bash
# 1. 检查 WireGuard
which wg-quick
wg-quick --version

# 2. 检查内核模块
lsmod | grep wireguard
sudo modprobe wireguard

# 3. 检查配置文件
sudo sed -E 's/^([[:space:]]*(PrivateKey|PresharedKey)[[:space:]]*=[[:space:]]*).*/\1<HIDDEN>/I' -- /etc/wireguard/str-dub303.conf

# 4. 手动测试连接
sudo wg-quick up str-dub303

# 5. 如果成功，测试状态
sudo wg show str-dub303

# 6. 断开
sudo wg-quick down str-dub303
```

## 🔍 配置文件验证

确保配置文件格式正确：

```bash
# 查看配置
sudo sed -E 's/^([[:space:]]*(PrivateKey|PresharedKey)[[:space:]]*=[[:space:]]*).*/\1<HIDDEN>/I' -- /etc/wireguard/str-dub303.conf

# 使用 v0.4 TUI 的导入/连接前校验；wg-quick strip 会输出私钥，不要用它制作日志。
```

**检查要点：**
- [ ] 有 `[Interface]` 部分
- [ ] 有 `[Peer]` 部分
- [ ] PrivateKey 不为空
- [ ] PublicKey 不为空
- [ ] Endpoint 格式正确（IP:端口）
- [ ] Address 格式正确（IP/掩码）

## 💡 快速修复

如果上述都没问题，尝试：

```bash
# 1. 重启网络
sudo systemctl restart NetworkManager

# 2. 重新加载模块
sudo modprobe -r wireguard
sudo modprobe wireguard

# 3. 重试连接
sudo wg-quick up str-dub303
```

## 🆘 如果还是失败

请运行并提供以下信息：

```bash
# 系统信息
uname -a
cat /etc/os-release | grep PRETTY_NAME

# WireGuard 信息
wg-quick --version
lsmod | grep wireguard

# 配置文件（同时隐藏 PrivateKey 与 PresharedKey）
sudo sed -E 's/^([[:space:]]*(PrivateKey|PresharedKey)[[:space:]]*=[[:space:]]*).*/\1<HIDDEN>/I' -- /etc/wireguard/str-dub303.conf

# 只描述错误类型和退出码，不要粘贴未经审查的完整 wg-quick 输出。
```

## 📝 常见错误及解决

| 错误信息 | 原因 | 解决方案 |
|----------|------|----------|
| `module not found` | 内核不支持 WireGuard | 安装 wireguard-dkms |
| `Permission denied` | 权限不足或 sudo 缓存失效 | 退出 TUI，运行 `sudo -v`，再以普通用户启动 |
| `Cannot allocate memory` | 资源不足 | 重启系统 |
| `Invalid argument` | 配置格式错误 | 重新下载配置 |
| `Address already in use` | 接口已存在 | 先 down 再 up |

---

**下一步：运行 `./test_connect.sh` 进行完整诊断**
