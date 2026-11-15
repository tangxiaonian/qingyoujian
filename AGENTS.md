# 轻右键项目协作规则

## 当前状态与授权边界

项目已获用户明确授权开发、测试，并在完成后新建公开 GitHub 仓库提交上传。允许为该次交付初始化 Git、提交和推送；公开前检查敏感信息与模板元数据。此前“仅文档”限制已由用户新指示解除。

当前目录已在本次用户授权下初始化 Git，开发分支为 `main`；原始来源为上游归档，Swift 参考草稿保留在本机并从版本控制中排除。开始任务先确认目录、Git 是否存在、当前分支及已有改动；没有仓库时如实说明，不擅自初始化、提交或清理文件。用户已有改动一律保留。

## 文档入口

- [docs/PRD.md](docs/PRD.md)：范围、行为规则、用户故事、验收。
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)：技术路线、模块、接口和待验证项。
- [docs/DB_DESIGN.md](docs/DB_DESIGN.md)：SQLite 结构、字段、种子与快照。
- [docs/PLAN.md](docs/PLAN.md)：M0 验证 → MVP → 迭代。

根目录的“产品规划.md”“技术选型调研.md”和上游 README 是历史背景；后续开发以以上四份基线文档及用户最新指示为准。文档之间有冲突先指出并统一，不自行选择扩大范围的版本。

## 技术栈与职责

- Vue 3、TypeScript、Vite：设置窗口，不承载本机业务真相。
- Tauri 2、Rust：动作宿主、领域逻辑、平台调用与数据访问。
- Objective-C、Finder Sync：最小 Finder 扩展与必要原生桥接。
- SQLite / rusqlite：本机持久化，无远程数据库。
- macOS 首发，Apple Silicon 首轮验证；使用系统 WebView，不携带 Node.js/Chromium 运行时。
- M0 的编译和原生逻辑检查已有记录；实际签名通信、沙箱读取与生命周期的验证状态以 docs/VALIDATION.md 为准。

## 目录约定

| 路径 | 用途与当前状态 |
| --- | --- |
| `docs/` | 当前文档；已有上游图片和 mockup，保留 |
| `src/App.vue` | 仅设置窗口布局、导航和全局提示；不堆页面表单或原生命令 |
| `src/views/`、`src/components/` | 通用设置、收藏、关于页面与共享开关、侧栏；编辑草稿归所属页面 |
| `src/composables/`、`src/services/` | 窗口级状态/事件生命周期与类型化 Tauri 接口；每个窗口只订阅一次 |
| `src-tauri/src/commands/` | 当前：薄 Tauri 命令入口 |
| `src-tauri/src/domain/` | 当前：上下文、文件、收藏、设置服务 |
| `src-tauri/src/storage/`、`migrations/` | 当前：SQLite 与版本化迁移 |
| `src-tauri/src/platform/macos/` | 当前：系统动作和事件桥 |
| `native/finder-extension/` | 当前：Objective-C 扩展及权限声明 |
| `tests/manual/` | 当前：真实 Finder 验收记录 |
| `Sources/`、`Resources/`、`Tests/` | 当前上游 Swift 草稿，仅参考，不是目标工程 |
| `scripts/` | 新脚本已使用 build-macos / install-local 等独立命名；旧脚本仅本机保留并排除上传 |

正式目录已建立；迁移位于 src-tauri/migrations/。原始参考目录保留但不参与构建。

## 构建与测试命令

截至 2026-11-15，正式工程、package-lock、Cargo.lock、Rust 工具链文件和新构建脚本已经建立。以下命令对应当前工程；验证结果见 docs/VALIDATION.md，不以命令存在推断检查通过。

| 命令（项目根目录） | 要求 |
| --- | --- |
| `npm ci` | 根据已提交/已提供的 package-lock 安装；使用仓库锁文件 |
| `npm run dev` | 启动前端开发；不安装或启用 Finder 扩展 |
| `npm run tauri dev` | Tauri 本机开发，不代表 Finder 集成完成 |
| `npm run typecheck` | vue-tsc，无输出产物 |
| `npm run lint` | ESLint |
| `npm run test -- --run` | Vitest 非 watch 模式 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Rust 格式检查 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Rust 静态检查 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 核心与临时 SQLite 测试 |
| `bash scripts/build-finder-extension.sh` | 仅编译扩展到构建目录，不安装 |
| `bash scripts/test-native.sh` | 原生菜单与命名对话框测试，会短暂显示自动关闭的自有测试窗口 |
| `QYJ_TEST_FINDER_HIDDEN=1 bash scripts/test-native.sh` | 额外实测 Finder 隐藏文件可见性往返并恢复，需已有辅助功能权限；须先说明影响 |
| `npm run build` | 当前：类型检查后构建生产前端 |
| `npm run build:macos` | 当前：扩展构建、Tauri 打包、嵌入及完整签名校验，不自动安装 |

旧 `scripts/build.sh`、`scripts/test.sh`、`install.sh`、`scripts/finder-menu` 对应 Swift 参考版。不要执行它们来冒充新工程验证，也不要在文档生成任务中运行。具体 Node/Rust 工具链版本由 M1 固定并记录，不能随意升级。

## 编码规范

- 用中文直接报告结论、证据、验证边界和未完成项，不把静态阅读或编译通过说成实机验证。
- 先追踪入口到最终效果，按模块职责修改；不把 SQL、系统命令或核心规则塞进 Vue 组件。
- Vue 页面共用 useAppState 的窗口级快照、动作锁和错误提示；Tauri 命令名与参数映射集中在 services/desktop.ts。共享控件只接收 props 并发出交互事件，不自行持久化。
- TypeScript 开启严格类型，避免无依据的 any；Rust 错误用 Result 传播，业务路径不用 unwrap/expect 代替错误处理。
- 新增或修改的 if、else、for（含增强 for）、while、do/while 语句体，即使只有一条语句，也必须使用 `{}`；按语言实际支持执行。
- 新增或修改的方法写有用的职责、约束或副作用注释；复杂逻辑补必要行注释，不逐行复述代码。
- 对外命令 snake_case，JSON camelCase，数据库 snake_case；字段与 docs/ARCHITECTURE.md 一致。
- 文件创建禁止覆盖；路径使用系统 API/独立参数传递，SQL 参数化；不把路径插入 Shell 字符串。
- 收藏更新使用事务；数据库是唯一真相，快照只用于扩展读取。变更存储结构必须新增迁移并更新文档。
- 右键菜单构建不能逐项检查收藏是否存在或扫描目录；UI 按需加载，关闭窗口须销毁 WebView。
- 不为几乎不会发生的问题增加复杂防护。确需额外防护时先说明实际失败方式、概率与代码成本，由用户决定；常规输入校验和已约定的不覆盖规则照文档执行。

## 验证规则

- 只运行与变更相关且有意义的检查；纯文档变更检查链接、命令状态、术语和跨文档一致性，不必构建应用。
- 逻辑测试使用临时目录，不连接用户真实数据库。首次应用全链路验证可使用本次新建的干净配置，只写入自建临时样例，结束后仅清理精确匹配的自建记录，不删除真实收藏。
- 前端测试不能证明 Finder 功能正常；必须覆盖真实空白处、文件、目录、多选与包文件。
- 资源占用用实测记录，不引用框架宣传数字作为产品结果。
- 修改系统偏好、重启 Finder、安装启用扩展、改变权限或剪贴板前说明影响，确认现有授权；隐藏状态/剪贴板测试后恢复原值。构建脚本不能暗含安装和重启动作。
- 检查通过后不无故重复或扩大测试；新增失败先处理第一个真实问题。

## 禁止事项

- 未获授权，不继续实现、安装、启用扩展、发布或操作系统设置。
- 不自动 git commit/push，不重置用户改动，不自动删除旧草稿或用户文件。
- 不擅自切换 Electron、Wails、全 Swift 或引入 Java 后端、远程数据库、后台 HTTP 服务。
- 不关闭 SIP、不注入 Finder、不伪造 Office 文件、不添加未要求的通用脚本执行功能。
- 不信任公开 URL 携带的动作；来源验证未完成时不得开放文件写入或终端动作。
- 不保存账号、密码、Token 到代码、文档或日志；不加入埋点、广告和联网依赖。
- 不删除上游 MIT 声明，不把上游参考代码或图片误称为独立完成的新产品。
