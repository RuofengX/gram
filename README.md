# Gram

一个包含多语言组件的 Telegram 相关工具集合，采用 Rust 工作区 + Python 扩展的方式组织：

- gram-core：核心能力与通用工具（Rust 库）
- gram-pytools：Python 工具与扩展（使用 maturin 构建的 PyO3 扩展）

本仓库使用 Cargo 工作区统一管理依赖与构建，并提供 GitHub Actions 工作流对 Python 扩展进行多平台打包发布。
