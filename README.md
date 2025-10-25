# Gram

一个包含多语言组件的 Telegram 相关工具集合，采用 Rust 工作区 + Python 扩展的方式组织：

- gram-core：核心能力与通用工具（Rust 库）
- gram-scraper：抓取/服务相关功能（Rust 可执行/服务）
- gram-type：类型与实体定义（Rust 库）
- gram-pytools：Python 工具与扩展（使用 maturin 构建的 PyO3 扩展）

本仓库使用 Cargo 工作区统一管理依赖与构建，并提供 GitHub Actions 工作流对 Python 扩展进行多平台打包发布。


## 快速开始

### 环境要求
- Rust（建议使用 rustup，稳定版即可）
- Python 3.8+（用于构建与开发 gram-pytools）
- 可选：Docker（如果使用 .devcontainer 进行开发）

### 拉取代码
```
# 你的常规 git 流程
```

### 配置环境变量（可选）
项目根目录可放置一个 `.env` 文件（已在 .gitignore 中忽略），用于本地开发时读取配置。例如：
```
TELEGRAM_API_ID=your_api_id
TELEGRAM_API_HASH=your_api_hash
DATABASE_URL=postgres://user:pass@localhost:5432/gram
```


## 构建与运行（Rust 工作区）
项目根目录下直接使用 Cargo：

- 构建全部 crate
```
cargo build
```

- 以发布模式构建
```
cargo build --release
```

- 运行可执行（以 gram-scraper 中的二进制为例）
```
cargo run -p gram-scraper -- <args>
```

> 提示：工作区成员在 `Cargo.toml` 顶层以 `members` 声明，具体可执行入口请查看 `gram-scraper/src/bin` 或服务端入口文件。


## Python 扩展（gram-pytools）
`gram-pytools` 使用 [maturin](https://github.com/PyO3/maturin) 基于 PyO3 构建。

- 开发模式安装（在项目根目录）
```
# 安装 maturin（若未安装）
pip install maturin

# 进入子目录进行开发安装
cd gram-pytools
maturin develop
```

- 构建 wheel 包
```
cd gram-pytools
maturin build --release
```

GitHub Actions 工作流位于 `.github/workflows/pytools.yml`，在多平台上构建 wheels 并可选发布到 PyPI（需要配置 `PYPI_API_TOKEN`）。


## 目录结构

```
/                      # 仓库根目录（本 README 所在位置）
├─ Cargo.toml          # 工作区配置
├─ gram-core/          # Rust 核心库
├─ gram-scraper/       # 抓取/服务相关可执行或服务
├─ gram-type/          # 类型与实体定义
├─ gram-pytools/       # Python 扩展（PyO3 + maturin）
├─ .github/workflows/  # CI 工作流（包含 pytools 打包）
└─ .devcontainer/      # 开发容器配置（可选）
```


## 开发提示
- 建议使用 `tracing` 获取结构化日志；`RUST_LOG` 环境变量可控制日志级别。
- 若使用数据库，`DATABASE_URL` 可通过 `.env` 提供，配合 `sea-orm` 使用。
- `tokio` 已启用 full 功能，异步运行时相关的示例请参考各子项目源码。


## 常见问题
- 构建失败：请确认 Rust 工具链已安装且版本较新（推荐 stable 最新）；对 Python 组件还需确保 Python 和 pip 环境可用。
- maturin 报错：确保已安装 `rustup`、`cargo`，并在 `gram-pytools` 目录下执行命令；Windows 下建议使用 VS Build Tools。
- 依赖解析冲突：工作区依赖集中在根 `Cargo.toml` 的 `[workspace.dependencies]`，必要时统一升级。


## 贡献
欢迎提交 Issue 与 PR。在提交代码前请：
- 运行本地构建与基本运行验证
- 遵循现有代码风格与模块划分


## 许可证
若未在各子项目另行声明，默认遵循本仓库许可证。


---

English (Brief)

Gram is a multi-language workspace combining Rust crates and a Python extension:
- gram-core (Rust lib), gram-scraper (Rust app/service), gram-type (Rust types), gram-pytools (PyO3 Python extension via maturin).

Build with Cargo at repo root; develop Python extension under gram-pytools using maturin. See CI at .github/workflows/pytools.yml for cross-platform wheel builds.
