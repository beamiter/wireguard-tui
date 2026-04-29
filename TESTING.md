# 测试指南

## 环境要求

- Linux 系统（Ubuntu 20.04+ 推荐）
- sudo 权限
- 网络连接
- Rust 1.70+ (如果从源码编译)

## 单元测试和集成测试

### 运行所有测试

```bash
cargo test
```

### 仅运行单元测试

```bash
cargo test --lib
```

### 仅运行集成测试

```bash
cargo test --test '*'
```

### 运行特定模块的测试

```bash
cargo test vpn::
cargo test download::
cargo test config::
```

### 查看测试输出

```bash
# 显示 println! 的输出
cargo test -- --nocapture

# 单线程运行（更清晰的输出）
cargo test -- --test-threads=1
```

## 手动测试

### 1. 安装检查

```bash
# 测试应用是否能检测 WireGuard 安装
./target/debug/wireguard-tui
```

**预期结果**：
- 应用启动而不崩溃
- 如果未安装 WireGuard，应该尝试自动安装
- 显示主界面

### 2. 配置下载测试

**步骤**：
1. 启动应用
2. 按 `r` 尝试下载配置
3. 输入凭证（如果提示）

**预期结果**：
- 应该显示 "Downloading..." 
- 配置文件下载到 `/etc/wireguard/`
- 列表显示已下载的服务器

### 3. VPN 连接测试

**步骤**：
1. 配置下载成功后
2. 用方向键选择一个服务器
3. 按 Enter 连接

**预期结果**：
- 显示 "Connected ✓"
- 服务器前显示 ● 符号
- 绿色连接指示

### 4. 状态监控测试

**步骤**：
1. 连接到 VPN 后
2. 按 `s` 查看状态

**预期结果**：
- 显示连接详情
- Received 和 Sent 流量不为零
- 显示正确的端点和 IP

### 5. 服务器切换测试

**步骤**：
1. 连接到服务器 A
2. 选择服务器 B
3. 按 Enter

**预期结果**：
- 自动从 A 断开
- 自动连接到 B
- 无需手动断开

### 6. 配置删除测试

**步骤**：
1. 选择任意配置
2. 按 `d` 删除

**预期结果**：
- 如果已连接，自动断开
- 配置文件从列表移除
- `/etc/wireguard/` 目录中文件被删除

## 压力测试

### 快速连接断开

```bash
# 重复连接和断开
for i in {1..10}; do
    echo "Iteration $i"
    # 模拟快速操作
    sleep 1
done
```

### 大量配置处理

1. 下载配置
2. 在列表中快速导航（↑↓ ↑↓ ↑↓）
3. 应用应该不卡顿

## 故障模拟测试

### 1. 网络断开

**步骤**：
1. 拔掉网线或禁用 WiFi
2. 尝试下载配置
3. 应该显示错误信息

**预期结果**：
- 应用不崩溃
- 显示明确的错误提示

### 2. 无效凭证

**步骤**：
1. 在 config.toml 中输入错误的凭证
2. 按 `r` 下载

**预期结果**：
- 下载失败
- 显示错误信息

### 3. WireGuard 不可用

**步骤**：
1. 卸载 WireGuard：`sudo apt remove wireguard`
2. 启动应用

**预期结果**：
- 应用尝试自动安装
- 如果安装失败，显示错误

### 4. 权限问题

**步骤**：
1. 不使用 sudo 运行应用
2. 尝试连接 VPN

**预期结果**：
- 提示输入 sudo 密码
- 或显示权限错误

## 性能测试

### 内存占用

```bash
# 启动应用并检查内存
./target/release/wireguard-tui &
PID=$!
while true; do
    ps aux | grep wireguard-tui | grep -v grep
    sleep 1
done
kill $PID
```

**预期**：
- 稳定在 10-20 MB
- 不应该持续增长

### CPU 占用

```bash
# 用 top 监控
top -p <PID>
```

**预期**：
- 空闲时 < 1%
- 下载时 < 10%
- 界面更新时 < 5%

## 网络测试

### 验证 IP 变更

**步骤**：
1. 运行 `curl -s https://api.ipify.org` 记录 IP1
2. 连接 VPN
3. 再次运行上述命令

**预期结果**：
- IP 已改变
- 匹配 VPN 提供商的 IP

### DNS 泄露测试

```bash
# 测试 DNS
nslookup google.com
```

**预期**：
- DNS 应该通过 VPN 路由
- 可以访问所有网站

## 日志和调试

### 启用调试日志

```bash
RUST_LOG=debug ./target/release/wireguard-tui
```

### 查看系统日志

```bash
# WireGuard 日志
sudo journalctl -u wireguard -f

# dmesg 日志
sudo dmesg | tail -50
```

## 清理测试环境

```bash
# 断开所有 VPN 连接
sudo wg-quick down <config-name>

# 清理配置文件
sudo rm -f /etc/wireguard/*.conf

# 卸载 WireGuard (如果需要)
sudo apt remove wireguard

# 清理编译产物
cargo clean
```

## 跨发行版测试清单

- [ ] Ubuntu 20.04
- [ ] Ubuntu 22.04
- [ ] Debian 11
- [ ] Debian 12
- [ ] Fedora 37+
- [ ] Arch Linux
- [ ] openSUSE Leap

## 测试报告模板

```markdown
# 测试报告

## 环境
- 操作系统: Ubuntu 22.04
- 内核版本: 6.8.0
- Rust 版本: 1.75.0

## 测试项目
- [ ] 安装和启动
- [ ] 配置下载
- [ ] VPN 连接
- [ ] 状态显示
- [ ] 服务器切换
- [ ] 配置删除

## 结果
- 通过: X/6
- 失败: Y/6

## 说明
...
```

## 自动化测试脚本

```bash
#!/bin/bash
# test_automation.sh

set -e

echo "=== WireGuard TUI Test Suite ==="
echo ""

# 编译
echo "1. Building..."
cargo build --release
echo "✓ Build successful"
echo ""

# 运行测试
echo "2. Running tests..."
cargo test -- --nocapture
echo "✓ Tests passed"
echo ""

# 检查格式
echo "3. Checking format..."
cargo fmt -- --check
echo "✓ Format correct"
echo ""

# Lint 检查
echo "4. Running clippy..."
cargo clippy -- -D warnings
echo "✓ No clippy warnings"
echo ""

echo "=== All tests passed! ==="
```

## 已知限制

1. 需要 sudo 权限
2. 仅支持 Linux
3. 需要活跃的网络连接
4. WireGuard 必须支持你的内核版本

## 故障排除

### 编译失败

```bash
# 清理并重新编译
cargo clean
cargo build --release

# 更新依赖
cargo update
```

### 运行时崩溃

```bash
# 启用日志
RUST_BACKTRACE=1 ./target/release/wireguard-tui

# 使用 debug 版本
cargo run
```

### 连接问题

```bash
# 检查 WireGuard 状态
sudo wg show

# 检查网络连接
ip route show

# 检查 DNS
systemd-resolve --status
```
