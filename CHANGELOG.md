# 更新日志 (Changelog)

## [v0.3.0] - 2026-04-29

### ✨ 重大更新：浏览器集成 + 导入功能

#### 🎯 核心变更
完全重新设计了配置文件获取方式：
- **移除**：自动 HTTP 下载功能
- **新增**：浏览器集成 + 手动导入流程

#### 🌐 浏览器集成
- **新快捷键 `o`**: 在默认浏览器打开下载页面
- 支持 Linux、macOS、Windows
- 自动打开 StrongVPN 下载页面
- 支持各种认证方式（验证码、2FA 等）

**实现:**
```rust
// download.rs
pub fn open_in_browser(&self) -> Result<()>
```

#### 📥 导入功能
- **新快捷键 `i`**: 导入下载的配置文件
- **新屏幕**: `Screen::Import` - 配置导入界面
- 自动扫描 `~/Downloads/` 目录
- 识别 `str-*.conf` 格式文件
- 支持单个或批量导入
- 按修改时间排序显示

**实现:**
```rust
// download.rs
pub fn scan_downloads(&self) -> Result<Vec<PathBuf>>
pub fn import_config(&self, source: &Path, target: &Path) -> Result<String>
pub fn import_configs(&self, sources: &[PathBuf], target: &Path) -> Result<Vec<String>>
```

#### 🎨 新界面

**导入界面:**
```
┌─ Import WireGuard Configurations ─────────────┐
│ Found 3 config(s) in ~/Downloads               │
│                                                │
│ ▶ str-us-001.conf (2048 bytes, 2 minutes ago) │
│   str-eu-002.conf (2056 bytes, 3 minutes ago) │
│   str-asia-101.conf (2032 bytes, 5 minutes ago)│
│                                                │
│ ↑↓: Select | Enter: Import | a: Import all    │
└────────────────────────────────────────────────┘
```

#### ⌨️ 快捷键变更

**主界面:**
- ✅ 新增 `o` - 打开浏览器下载页面
- ✅ 新增 `i` - 导入配置文件
- ❌ 移除 `r` - 不再自动下载

**导入界面:**
- `↑↓` - 选择文件
- `Enter` - 导入选中
- `a` - 导入全部
- `Esc` - 返回

#### 📄 文件变更

**修改:**
- `src/download.rs` - 完全重写，移除 HTTP 下载，添加浏览器和导入
- `src/app.rs` - 新增导入相关字段和处理函数
- `src/main.rs` - 更新键盘事件处理
- `src/ui.rs` - 新增导入界面渲染

**新增字段:**
```rust
pub struct App {
    // ...
    pub import_configs: Vec<PathBuf>,
    pub import_selected: usize,
}
```

**新增方法:**
```rust
// app.rs
pub async fn handle_open_browser(&mut self) -> Result<()>
pub async fn handle_import(&mut self) -> Result<()>
pub async fn handle_import_selected(&mut self) -> Result<()>
pub async fn handle_import_all(&mut self) -> Result<()>
```

#### 📚 新增文档
- `BROWSER_IMPORT.md` - 完整的浏览器集成和导入功能文档

#### 🔄 工作流程对比

**之前 (v0.2.0):**
```
按 'r' → 自动下载所有配置 → 使用
```

**现在 (v0.3.0):**
```
按 'o' → 浏览器手动下载 → 按 'i' 导入 → 使用
```

#### 💡 优势

- ✅ **更灵活**: 手动选择需要的服务器
- ✅ **更可靠**: 支持验证码、2FA 等认证
- ✅ **更安全**: 用户完全控制下载过程
- ✅ **更简单**: 不需要处理 HTTP 认证
- ✅ **更通用**: 适用于任何需要手动登录的场景

#### ⚠️ 破坏性变更

**移除的功能:**
- `handle_refresh()` 方法
- `download_all_configs()` 方法
- `download_config()` 方法
- `list_available_configs()` 方法

**依赖变更:**
- 不再需要 `reqwest` (HTTP 客户端)
- 不再需要 `base64` (认证编码)
- 不再需要 `regex` (HTML 解析)

#### 🐛 修复
- 修复了自动下载时的认证问题
- 修复了 HTTP 超时问题
- 改进了错误处理

---

## [v0.2.0] - 2026-04-29

### ✨ 新增功能

#### 🔧 自动配置生成
- **配置文件自动创建**: 首次运行时自动生成 `config.toml` 模板
- **智能检测**: 自动检测凭证是否为模板默认值
- **友好提示**: 在界面显示配置文件路径和状态
- **详细注释**: 配置文件包含完整的使用说明

**影响:**
- 用户无需手动创建配置文件
- 更好的首次使用体验
- 减少配置错误

**文件变更:**
- `src/config.rs`: 新增 `ensure_config_exists()` 和 `create_config_template()`
- `src/app.rs`: 新增凭证状态检测
- `src/ui.rs`: 改进设置界面，显示配置状态

#### 📝 改进的设置界面
- 显示配置文件的完整路径
- 显示凭证配置状态（已配置/未配置）
- 检测是否使用模板默认值
- 提供详细的配置指导

**新界面特性:**
```
Configuration Status
Status: ⚠ Not Configured

Username: ⚠ Using template value - please update!
Password: ⚠ Using template value - please update!

Configuration File Location:
  /home/user/.config/wireguard-tui/config.toml

How to configure:
  1. Edit the file: nano /path/to/config.toml
  2. Update the username and password...
  3. Save and restart the application
```

#### 📚 新增文档
- `AUTO_CONFIG.md`: 自动配置功能完整文档
- `test_auto_config.sh`: 自动配置测试脚本
- `CHANGELOG.md`: 本文件，记录版本变更

### 🔄 改进

#### 用户体验
- **首次运行提示**: 如果检测到模板凭证，显示配置路径
- **消息持久化**: 重要提示会持续显示
- **颜色编码**: 使用颜色区分配置状态（绿色=已配置，红色=未配置）

#### 文档更新
- `README.md`: 更新配置部分，强调自动生成功能
- `QUICKSTART.md`: 简化配置步骤说明
- `setup.sh`: 更新安装脚本说明

### 🐛 修复

- 修复未使用的 `mut` 警告
- 修复 `base64` API 弃用警告
- 修复 `Frame<B>` 泛型参数问题

### 📊 技术细节

**新增方法:**
```rust
// config.rs
fn ensure_config_exists(&self) -> Result<()>
fn create_config_template(&self) -> Result<()>
pub fn get_config_path_str(&self) -> String

// app.rs
// 在 new() 中添加凭证检测逻辑
```

**依赖更新:**
- 使用 `base64::engine::general_purpose::STANDARD` 替代废弃的 `base64::encode`
- ratatui 从 `Frame<B>` 更新为 `Frame`

---

## [v0.1.0] - 2026-04-29

### ✨ 初始版本

#### 核心功能
- 🔒 WireGuard 自动安装（支持 Ubuntu, Debian, Fedora, Arch, openSUSE）
- 📥 从 StrongVPN 下载配置文件
- 🔌 VPN 连接/断开管理
- 🔄 服务器无缝切换
- 📊 实时流量和状态监控
- 🗂️ 配置文件管理（列表、删除）
- 💻 交互式 TUI 界面

#### 技术栈
- **ratatui**: TUI 框架
- **tokio**: 异步运行时
- **reqwest**: HTTP 客户端
- **crossterm**: 终端操作
- **serde**: 配置序列化

#### 项目结构
```
wireguard-tui/
├── src/
│   ├── main.rs       # 主入口、事件循环
│   ├── app.rs        # 应用状态管理
│   ├── ui.rs         # TUI 渲染
│   ├── vpn.rs        # VPN 操作
│   ├── download.rs   # 配置下载
│   ├── config.rs     # 配置管理
│   └── commands.rs   # 系统命令
├── README.md
├── QUICKSTART.md
├── TESTING.md
├── PROJECT_SUMMARY.md
└── setup.sh
```

#### 快捷键
- `↑/↓` - 导航
- `Enter` - 连接/断开
- `r` - 下载配置
- `d` - 删除配置
- `s` - 查看状态
- `q` - 退出

#### 文档
- 完整的 README
- 5分钟快速开始指南
- 测试和调试文档
- 项目架构总结

---

## 计划中的功能

### v0.3.0 (未来)
- [ ] 配置文件编辑器（内置）
- [ ] 服务器搜索/过滤
- [ ] 连接历史记录
- [ ] 自动重连
- [ ] 系统托盘图标

### v0.4.0 (未来)
- [ ] 服务器速度测试
- [ ] 按地区分组显示
- [ ] 收藏服务器
- [ ] 连接日志
- [ ] 流量限制设置

### v1.0.0 (目标)
- [ ] macOS 支持
- [ ] Windows 支持
- [ ] systemd 服务
- [ ] 多配置文件支持
- [ ] GUI 版本

---

## 版本说明

### 版本号规则
使用语义化版本 (Semantic Versioning):
- **主版本号**: 不兼容的 API 变更
- **次版本号**: 向后兼容的功能性新增
- **修订号**: 向后兼容的问题修正

### 发布周期
- 修复版本: 随时发布
- 功能版本: 每月一次
- 主版本: 按需发布

---

## 贡献者

- **开发**: Claude (Anthropic)
- **需求**: 用户 (mm)

## 许可证

MIT License
