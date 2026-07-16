# ✅ 复选框多选导入功能

> [!WARNING]
> 这是已归档的早期界面记录，不是 v0.4 操作说明；请以当前 [README.md](README.md) 为准。

## 功能说明

按 `i` 导入时，会列出 Downloads 下**所有 .conf 文件**，支持多选勾选导入。

## 🎯 导入界面

```
┌─ Import WireGuard Configurations ─────────────────────────────────┐
│                                                                    │
│ Found 5 config(s) - 3 selected                                    │
│                                                                    │
│ ▶ [✓] str-us-001.conf (2048 bytes, 2 minutes ago)                │
│   [✓] str-eu-002.conf (2056 bytes, 3 minutes ago)                │
│   [ ] str-asia-101.conf (2032 bytes, 5 minutes ago)              │
│   [✓] myserver.conf (1998 bytes, 10 minutes ago)                 │
│   [ ] test.conf (2100 bytes, 1 hour ago)                         │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘

↑↓: Navigate | Space: Check/Uncheck | a: Check All | n: Uncheck All | Enter: Import | Esc: Cancel
```

**说明：**
- `▶` = 当前选中项（光标位置）
- `[✓]` = 已勾选（将被导入）
- `[ ]` = 未勾选（不导入）
- 绿色 = 已勾选的文件
- 白色 = 未勾选的文件

## ⌨️ 操作快捷键

| 键 | 功能 |
|----|------|
| `↑` `↓` | 上下移动光标 |
| `Space` | 勾选/取消勾选当前项 |
| `a` | 全选（勾选所有文件） |
| `n` | 全不选（取消所有勾选） |
| `Enter` | 导入所有勾选的文件 |
| `Esc` | 取消并返回 |

## 📋 使用流程

### 完整操作示例

```
1. 按 'i' 进入导入界面
   ↓
2. 使用 ↑↓ 移动到想要的文件
   ↓
3. 按 Space 切换勾选状态
   ↓
4. 重复步骤 2-3 选择多个文件
   ↓
5. 按 Enter 导入所有勾选的文件
   ↓
6. 导入完成，返回主界面
```

### 快速操作

**导入所有文件：**
```
按 'i' → 按 'a' 全选 → 按 Enter
```

**只导入一个文件：**
```
按 'i' → 按 'n' 全不选 → 移动到文件 → 按 Space → 按 Enter
```

**导入部分文件：**
```
按 'i' → 按 'n' 全不选 → 逐个 Space 勾选 → 按 Enter
```

## 🎨 界面特点

### 颜色编码

- **绿色** - 已勾选的文件（将被导入）
- **白色** - 未勾选的文件
- **灰色背景** - 当前光标所在行

### 状态显示

标题栏显示：
```
Found 5 config(s) - 3 selected
```
- 左边：总共找到的文件数
- 右边：当前勾选的文件数

### 复选框样式

```
[✓]  已勾选
[ ]  未勾选
```

## 📂 文件检测规则

**扫描位置：** `~/Downloads/`

**检测规则：**
- ✅ 所有 `.conf` 文件
- ✅ 不限制文件名格式
- ✅ 按修改时间排序（最新的在上面）

**示例文件名：**
```
✓ str-us-001.conf      （StrongVPN 标准格式）
✓ str-zrh302.conf      （StrongVPN 标准格式）
✓ myserver.conf        （自定义名称）
✓ vpn-config.conf      （自定义名称）
✓ test.conf            （任何 .conf 文件）
```

## 🎯 实际使用场景

### 场景 1：选择性导入

你下载了 10 个服务器配置，但只想导入 3 个：

```
1. 按 'i' 查看列表
2. 按 'n' 全部取消勾选
3. 用 ↑↓ 找到需要的服务器
4. 按 Space 勾选
5. 重复找到另外两个并勾选
6. 按 Enter 导入这 3 个
```

### 场景 2：批量导入

下载了很多配置，全部导入：

```
1. 按 'i' 查看列表
2. 按 'a' 全选（或默认已全选）
3. 按 Enter 导入全部
```

### 场景 3：排除某些文件

大部分要导入，只有几个不要：

```
1. 按 'i' 查看列表（默认全选）
2. 用 ↑↓ 找到不要的文件
3. 按 Space 取消勾选
4. 重复找其他不要的
5. 按 Enter 导入其余的
```

## 💡 提示与技巧

### 提示 1：默认全选

进入导入界面时，默认**所有文件都已勾选**，方便快速导入。

### 提示 2：实时计数

标题栏会实时显示勾选的文件数量，方便确认。

### 提示 3：快速清空

如果要从零开始选择，先按 `n` 全部取消勾选。

### 提示 4：文件信息

每个文件显示：
- 文件名
- 文件大小
- 修改时间（多久前下载的）

### 提示 5：无需勾选也能返回

如果改变主意不想导入了，按 `Esc` 直接返回。

## 🔍 常见问题

### Q: 必须勾选才能导入吗？

**A:** 是的。按 Enter 只会导入勾选的文件。如果没有勾选任何文件，会显示错误提示。

### Q: 默认是全选还是全不选？

**A:** 默认**全选**，这样如果你想导入所有文件，直接按 Enter 即可。

### Q: Space 键没反应？

**A:** 确保：
1. 你在导入界面（按过 `i`）
2. 光标在某个文件上
3. 文件列表不为空

### Q: 能看到哪些文件会被导入吗？

**A:** 可以。标题显示选中数量，文件列表中绿色的就是会被导入的。

### Q: 导入后原文件会删除吗？

**A:** 不会。原文件保留在 `~/Downloads/`，只是复制到 `/etc/wireguard/`。

### Q: 能导入非 WireGuard 的 .conf 文件吗？

**A:** 可以显示和选择，但导入后能否正常使用取决于文件内容是否是有效的 WireGuard 配置。

## 🛠️ 技术实现

### 数据结构

```rust
pub struct App {
    pub import_configs: Vec<PathBuf>,    // 文件列表
    pub import_selected: usize,          // 光标位置
    pub import_checked: Vec<bool>,       // 勾选状态
}
```

### 勾选逻辑

```rust
// 切换当前项
pub fn handle_toggle_check(&mut self) {
    if self.import_selected < self.import_checked.len() {
        self.import_checked[self.import_selected] = 
            !self.import_checked[self.import_selected];
    }
}

// 全选
for i in 0..app.import_checked.len() {
    app.import_checked[i] = true;
}

// 全不选
for i in 0..app.import_checked.len() {
    app.import_checked[i] = false;
}
```

### 导入逻辑

```rust
// 只导入勾选的文件
let selected_paths: Vec<_> = self.import_configs
    .iter()
    .enumerate()
    .filter(|(idx, _)| self.import_checked.get(*idx).copied().unwrap_or(false))
    .map(|(_, path)| path.clone())
    .collect();
```

## 📊 状态说明

### 界面状态

| 状态 | 说明 |
|------|------|
| 加载中 | 正在扫描 Downloads 目录 |
| 空列表 | 没有找到 .conf 文件 |
| 显示列表 | 找到文件，可以勾选 |
| 导入中 | 正在复制文件到 /etc/wireguard |
| 完成 | 返回主界面 |

### 复选框状态

```
初始状态：全部勾选 [✓]
↓
用户操作：Space 切换
↓
最终状态：部分勾选
↓
Enter：导入勾选的文件
```

## 🎓 最佳实践

### 1. 先查看再选择

不要着急按 Enter，先看看列表中有哪些文件。

### 2. 使用全选/全不选

- 如果大部分要导入 → 保持默认全选，取消不要的
- 如果大部分不要 → 按 `n` 全不选，勾选要的

### 3. 检查文件名

确保文件名正确，避免导入错误的配置。

### 4. 查看修改时间

最新下载的文件在上面，方便找到刚下载的配置。

### 5. 确认数量

导入前看标题栏的选中数量，确保符合预期。

## 📝 示例会话

```
用户操作                  界面显示
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

按 'i'              →   Found 3 config(s) - 3 selected
                        ▶ [✓] str-us-001.conf
                          [✓] str-eu-002.conf
                          [✓] test.conf

按 'n' (全不选)      →   Found 3 config(s) - 0 selected
                        ▶ [ ] str-us-001.conf
                          [ ] str-eu-002.conf
                          [ ] test.conf

按 Space (勾选)     →   Found 3 config(s) - 1 selected
                        ▶ [✓] str-us-001.conf
                          [ ] str-eu-002.conf
                          [ ] test.conf

按 ↓ (下移)         →   Found 3 config(s) - 1 selected
                          [✓] str-us-001.conf
                        ▶ [ ] str-eu-002.conf
                          [ ] test.conf

按 Space (勾选)     →   Found 3 config(s) - 2 selected
                          [✓] str-us-001.conf
                        ▶ [✓] str-eu-002.conf
                          [ ] test.conf

按 Enter (导入)     →   ✓ Imported 2 config(s)
                        返回主界面
```

## 🔄 更新内容

### v0.3.0 最终版

**新增：**
- ✅ 复选框多选功能
- ✅ Space 键切换勾选
- ✅ 全选/全不选快捷键
- ✅ 实时显示选中数量
- ✅ 颜色区分勾选状态
- ✅ 支持所有 .conf 文件（不限格式）

**改进：**
- ✅ 默认全选，方便快速导入
- ✅ 只导入勾选的文件
- ✅ 更清晰的界面提示

---

**版本：** v0.3.0 (多选导入)  
**状态：** ✅ 已实现并测试  
**文档：** 完整功能说明
