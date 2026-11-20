# 轻右键本地数据设计

版本：1.0；日期：2026-11-14。实现基线：四表已成为迁移 001，新增文件操作菜单为迁移 002；当前 schema version 为 6。正式运行数据库仅存在本机，不上传仓库。

## 1. 存储原则

使用嵌入式 SQLite，不需要 MySQL、账号或数据库服务进程。收藏需要稳定排序、去重和事务更新，SQLite 便于集中维护这些规则。数据库不是独立后台服务。

- 拟定目录：`~/Library/Application Support/local.qingyoujian/`。
- 数据库：`qingyoujian.sqlite3`，路径由 Rust 数据目录适配器解析，统一供开发和发布调用；测试使用临时目录。
- 访问：只有 AppHost 的 Rust Repository；Vue 和 Finder 扩展不直接连接数据库。
- 迁移：`src-tauri/migrations/NNN_description.sql`，用 `PRAGMA user_version` 标识版本。按版本顺序在事务内执行，不在启动时删除重建已有数据库。
- 时间：UTC Unix 毫秒，整数；实体 ID 使用 UUID 字符串，内置文件类型用固定 ID。
- 删除：收藏直接物理删除，不需要逻辑删除、审计流水或同步字段。
- 默认卸载保留配置；清除数据必须由用户明确选择。

## 2. 表与关系

| 表 | 用途 | 主键 | 主要约束 |
| --- | --- | --- | --- |
| `favorite_folder` | 收藏目录和展示顺序 | `id` | 标准化路径唯一 |
| `file_type` | 内置类型及以后自定义文本类型 | `id` | `type_key` 唯一 |
| `menu_item` | 顶层菜单开关和顺序 | `id` | 动作 ID 固定白名单 |
| `app_setting` | 少量应用级设置 | `key` | 每键一条 JSON 值 |

四张表目前没有一对多的业务从属关系，不强行添加外键或中间表。关联发生在动作层：`resourceId` 为文件类型 ID 或收藏 ID；宿主分别查对应表。菜单顶层 `new-file` 和 `favorites` 通过服务聚合相应列表，不存逗号分隔的子项 ID。

实际用户文件、目录及 Finder 的隐藏显示状态均不存入数据库。常用目录只保存引用路径；文件夹不是本应用拥有的实体，没有级联删除文件系统的行为。

## 3. 表结构与字段

### 3.1 favorite_folder

| 字段 | SQLite 类型 | 说明 |
| --- | --- | --- |
| `id` | TEXT，主键 | 收藏 UUID，改名、排序、重新定位后保持不变 |
| `display_name` | TEXT，非空 | 收藏名称，不修改目录真实名称 |
| `normalized_path` | TEXT，非空、唯一 | 绝对 POSIX 路径，作为访问路径和去重键 |
| `sort_order` | INTEGER，非空 | 从 0 开始，展示按该字段、id 排序 |
| `created_at` | INTEGER，非空 | 创建时间 |
| `updated_at` | INTEGER，非空 | 最近一次改名、排序或重新定位时间 |

路径标准化由 macOS 适配器完成：绝对化、消除冗余分隔符和 `.` / `..`，根目录保留 `/`；不统一小写，不自动解析符号链接。大小写规则由实际文件系统决定，MVP 只对同一标准化路径去重。添加及重新定位时验证其为可用目录；读取列表时不逐条检查磁盘。

索引：唯一索引保护重复路径；`(sort_order, id)` 支持排序列表。收藏不做全文检索或目录递归索引。

### 3.2 file_type

| 字段 | SQLite 类型 | 说明 |
| --- | --- | --- |
| `id` | TEXT，主键 | 例如 `builtin-txt`、`builtin-docx` |
| `type_key` | TEXT，非空、唯一 | 稳定业务键：txt / markdown / json / xml / shell / word / excel |
| `display_name` | TEXT，非空 | 菜单显示名称 |
| `extension` | TEXT，非空 | 不含前导点，例如 md |
| `default_basename` | TEXT，非空 | 默认文件基础名称，例如“未命名” |
| `initial_content` | TEXT，非空 | UTF-8 初始文本，允许空串 |
| `enabled` | INTEGER，非空 | 0 / 1 |
| `is_builtin` | INTEGER，非空 | 0 / 1，MVP 全为 1 |
| `sort_order` | INTEGER，非空 | 子菜单顺序 |
| `created_at` / `updated_at` | INTEGER，非空 | 创建/修改时间 |

同扩展名未来可有多个模板，因此 `extension` 不设唯一。MVP 不提供任意模板编辑接口。Office 类型的 initial_content 为空；实际字节使用编译期嵌入的 assets/templates/blank.docx、blank.xlsx，不将二进制内容塞入 TEXT。

### 3.3 menu_item

| 字段 | SQLite 类型 | 说明 |
| --- | --- | --- |
| `id` | TEXT，主键 | 固定动作 ID |
| `enabled` | INTEGER，非空 | 0 / 1 |
| `sort_order` | INTEGER，非空 | 顶层菜单顺序 |
| `updated_at` | INTEGER，非空 | 最近设置时间 |

固定 ID：`new-folder`、`new-file`、`copy-path`、`copy-to`、`move-to`、`trash`、`terminal`、`toggle-hidden`、`favorites`、`add-favorite`、`compress-zip`、`extract-zip-here`、`extract-zip-folder`。不在表中存本地化文案、选择器或可执行命令；应用内映射文案和实现。

### 3.4 app_setting

| 字段 | SQLite 类型 | 说明 |
| --- | --- | --- |
| `key` | TEXT，主键 | 固定设置键，不接收未知键 |
| `value_json` | TEXT，非空 | Rust 按键校验的 JSON 值 |
| `updated_at` | INTEGER，非空 | 最近更新时间 |

| 设置键 | JSON 类型 | 初始值/含义 |
| --- | --- | --- |
| `terminal_bundle_id` | string | `"com.apple.Terminal"`；MVP 只支持系统 Terminal |
| `show_menu_bar_icon` | boolean | false；只控制菜单栏图标 |
| `start_at_login` | boolean | false；用户期望值，实际授权另查系统 |
| `run_in_background` | boolean | true；设置关闭后的宿主驻留策略 |
| `settings_revision` | number，非负整数 | `1`；配置事务递增，发布为快照 revision |

不保存 `AppleShowAllFiles`，避免与 Finder 真正状态冲突。界面语言和主题首版跟随系统，无需另建设置键。

## 4. 建表 SQL 草案

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE favorite_folder (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    normalized_path TEXT NOT NULL CHECK (substr(normalized_path, 1, 1) = '/'),
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (normalized_path)
);
CREATE INDEX idx_favorite_order ON favorite_folder(sort_order, id);

CREATE TABLE file_type (
    id TEXT PRIMARY KEY NOT NULL,
    type_key TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    extension TEXT NOT NULL CHECK (length(extension) > 0),
    default_basename TEXT NOT NULL CHECK (length(trim(default_basename)) > 0),
    initial_content TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    is_builtin INTEGER NOT NULL DEFAULT 1 CHECK (is_builtin IN (0, 1)),
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE menu_item (
    id TEXT PRIMARY KEY NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    updated_at INTEGER NOT NULL
);

CREATE TABLE app_setting (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
```

`PRAGMA foreign_keys` 为连接初始化约定；当前无外键。菜单和类型数量很少，无需为每个查询另建索引。文件名字符和 JSON 类型由服务校验，数据库约束作为基础结构约束。

## 5. 初始化与写入规则

第一次启动在单一初始化事务中建表、写默认配置和内置类型，设置初始 `user_version = 1`；随后应用迁移 002，将版本提升为 2，插入复制、移动、废纸篓菜单，保留原有收藏和设置。后续启动不覆盖用户排序、菜单开关和收藏。

| 内置 ID | 类型键 | 扩展名 | 初始内容 | 顺序 |
| --- | --- | --- | --- | --- |
| `builtin-txt` | txt | txt | 空串 | 0 |
| `builtin-md` | markdown | md | 空串 | 1 |
| `builtin-json` | json | json | `{}` 加换行 | 2 |
| `builtin-xml` | xml | xml | XML 声明 + 换行 + `<root/>` + 换行 | 3 |
| `builtin-sh` | shell | sh | `#!/bin/sh` 加换行 | 4 |
| `builtin-docx` | word | docx | 包内 Word 模板字节 | 5 |
| `builtin-xlsx` | excel | xlsx | 包内 Excel 模板字节 | 6 |

各类型默认基础名为“未命名”，显示名称对应 PRD。初始七个菜单按 PRD 顺序初始化，迁移 002 在路径复制后插入三个新菜单，共十个菜单，全部启用。自定义文件名是应用固定菜单动作，不额外存为文件类型记录。

收藏新增放末尾；排序请求必须包含当前完整 ID 集合，事务内更新顺序；移除后允许顺序有间隔，无需额外维护稠密编号。修改配置与递增 `settings_revision` 在同一事务内完成。SQL 使用参数绑定。

## 6. 扩展快照及关联

SQLite 提交 → 读取已提交的配置与 revision → 构建完整快照 → 发布至 CFPreferences 的 `MenuSnapshot` → 通知设置 UI。结构见 [ARCHITECTURE.md](ARCHITECTURE.md)。

快照不是第五张业务表，也不是双向数据库；扩展只读，丢失可从 SQLite 重建。包含菜单、启用的文件类型及收藏的 ID/名称/路径，不含初始文件内容、可执行命令或用户文件内容。发布失败按架构文档反馈，避免“数据已保存”与“菜单已更新”混淆。

## 7. 数据生命周期与验证

数据库迁移测试使用临时目录，验证首次初始化、重复启动不覆盖、升级事务失败保留旧版本。收藏测试验证去重、重命名不改路径、重定位不改 ID、排序和移除不操作真实文件夹。

数据库打不开时给出错误，不自动删除或覆盖。MVP 不建立同步队列、通用备份平台、审计日志表或复杂自动修复；若后续确有需求再设计。

## 8. 迁移 003：新建文件夹

将原有菜单整体顺序后移一位，插入 new-folder，更新快照 revision 和 user_version=3；不改动收藏、文件类型或已有菜单开关。迁移 003 后共十一个顶层菜单项；迁移 005 后合并为十项。

## 9. 迁移 004：启动与后台偏好

插入三个独立布尔设置，初始值为 false / false / true；不在迁移或启动时自动注册登录项。实际登录项状态来自 SMAppService。该迁移只增加键、更新 revision 并设置 user_version=4，不改用户收藏和菜单开关。

### 迁移 005：即时隐藏文件切换

将 show-hidden / hide-hidden 合并为 toggle-hidden，enabled 取原两项最大值（两项都关闭则保持关闭），sort_order 取原两项最小值，更新 revision 并设置 user_version=5。收藏、文件类型与其他菜单设置不变。隐藏文件的系统实时显示状态不落库。

### 迁移 006：ZIP 菜单（2026-11-20）

在已有菜单最大 sort_order 后依次追加 compress-zip、extract-zip-here、extract-zip-folder，默认 enabled=1，更新 revision 并设置 user_version=6。目前共十三个顶层菜单配置。只添加菜单行，不重写已有菜单顺序或开关，也不修改收藏、模板或启动偏好。ZIP 文件内容、操作历史及密码均不入库；不新增数据表。
