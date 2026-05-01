# plugin_doro

[luo9_bot](https://github.com/luoy-oss/luo9_bot) 的 Doro 每日结局插件，使用 Rust 编写。

每天发送指令，获取属于你的专属 Doro 结局图片。同一天内同一用户的结果固定，次日刷新。

## 功能

- **今日doro结局** — 获取今日的 Doro 结局（每日随机，每用户固定）
- **doro结局帮助** — 查看帮助信息
- **doro add** — 添加新结局（管理员）
- **doro remove** — 删除结局（管理员）
- **doro update** — 修改结局名称（管理员）
- **doro list** — 列出所有结局（管理员）

> 仅支持群聊，不支持私聊。

## 安装

### 从 Release 下载

从 [Releases](https://github.com/luo9-bot/plugin_doro/releases) 页面下载对应平台的文件：

| 平台 | 文件 |
|------|------|
| Linux | `libplugin_doro.so` |
| Windows | `plugin_doro.dll` |

将文件放入 luo9_bot 的插件目录即可。

### 从源码编译

```bash
git clone https://github.com/luo9-bot/plugin_doro.git
cd plugin_doro
cargo build --release
```

产物位于 `target/release/` 目录。

## 使用

### 普通用户

| 指令 | 说明 |
|------|------|
| `今日doro结局` | 获取今日专属结局图片 |
| `doro结局帮助` | 查看帮助 |

### 管理员

| 指令 | 说明 |
|------|------|
| `doro add <中文名> <英文名> [图片URL]` | 添加新结局 |
| `doro remove <ID或中文名>` | 删除结局 |
| `doro update <ID> <1\|2> <新值>` | 修改结局（1=中文名，2=英文名） |
| `doro list` | 列出所有结局 |

## 工作原理

1. **首次启动**时自动从 GitHub/Gitee 双源下载结局图片和元数据
2. 用户发送 `今日doro结局` 后，插件随机分配一个结局并记录
3. 同一天内再次发送，返回相同结局
4. 次日自动刷新，重新随机

## 数据存储

所有数据存储在运行目录下的 `data/plugin_doro/`：

```
data/plugin_doro/
├── doroendings.json          # 结局元数据
├── doro_date_record.json     # 日期记录
├── user_doro_map.json        # 用户-结局映射
└── DoroEndingPic/            # 结局图片目录
    ├── 00000001_OrangeEnd.jpg
    ├── 00000002_RaceEnd.jpg
    └── ...
```

## 项目结构

```
src/
├── core.rs         # 插件入口 plugin_main()
├── lib.rs          # 消息路由与命令处理
├── doro.rs         # 结局数据模型与管理器
└── downloader.rs   # GitHub/Gitee 双源资源下载器
```

## 依赖

- [luo9_sdk](https://crates.io/crates/luo9_sdk) — luo9_bot 插件 SDK
- [serde](https://crates.io/crates/serde) / [serde_json](https://crates.io/crates/serde_json) — JSON 序列化
- [ureq](https://crates.io/crates/ureq) — 同步 HTTP 客户端

## 许可证

[MIT](LICENSE)
