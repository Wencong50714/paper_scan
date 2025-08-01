## Quick Start

### 1. 安装依赖

确保你已安装 Rust 工具链（推荐使用 [rustup](https://rustup.rs/)）。

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

进入项目目录，安装依赖：

```bash
cd paper_scan
cargo build --release
```

### 2. 配置环境变量

创建 `.env` 文件并配置 API 访问：

```bash
cp .env.example .env
```

编辑 `.env` 文件，填入你的 API 配置：

```bash
# OpenAI API Configuration
BASE_URL=https://api.openai.com/v1
API_KEY=your_openai_api_key_here
```

#### 支持的 API 格式
本工具使用 OpenAI 格式的 API 请求，支持：
- OpenAI 官方 API
- 任何兼容 OpenAI 格式的 LLM 服务（如 Azure OpenAI、本地部署的模型等）

#### 环境变量说明
- `BASE_URL`: API 基础地址（默认：https://api.openai.com/v1）
- `API_KEY`: 你的 API 密钥

### 3. 运行

#### 单个论文 URL 处理

```bash
cargo run --release -- single <arxiv_url>
```
例如：
```bash
cargo run --release -- single https://arxiv.org/abs/1234.5678
```

处理完成后，会在当前目录生成一个 `tex/{}.tex` 文件，包含生成的论文笔记。

#### 批量处理（从文件读取 URL）

```bash
cargo run --release -- batch <file_path>
```
例如：
```bash
cargo run --release -- batch urls.txt
```

文件格式：每行一个 arXiv 论文链接。

#### 复制 PDF

```bash
cargo run --release -- collect-pdf
```

将 tex 文件夹下编译好的 pdf 文件移动到 pdfs 文件夹下

### 4. 输出文件

生成的笔记将以 LaTeX 格式保存，文件名格式为 `{paper_id}.tex`，例如：
- `2401.12345.tex`
- `2309.67890.tex`

### 5. 自定义提示词

系统使用 `prompts.txt` 文件中的内容作为生成笔记的提示词。你可以根据需要修改此文件来自定义笔记的格式和内容要求。

### 6. 编译 LaTeX 文件

生成 `.tex` 文件后，可以使用任何 LaTeX 编译器进行编译：

```bash
pdflatex 2401.12345.tex
```

## 技术架构

本工具采用模块化设计：
- **下载器 (downloader)**: 从 arXiv 下载论文源码
- **提取器 (extractor)**: 解压并提取论文内容
- **处理器 (processor)**: 解析 TeX 文件，提取结构化信息
- **LLM客户端 (llm_client)**: 与 OpenAI 格式 API 交互
- **笔记生成器 (note_generator)**: 生成格式化的 LaTeX 笔记
