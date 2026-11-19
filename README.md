# SHIT-Pool 石库

访客可以按题干或来源检索，用左侧分类和右侧多选标签筛选。结果每页 10 道，点击题目可查看 ID、答案、来源、分类、标签及备注，并下载包含配图的 TeX ZIP。题库写操作通过带鉴权的 API 或 CLI 完成，不提供网页管理入口。

顶部「套题」按来源列出试卷。选择来源后展示该试卷已录入的全部题目，不分页。

顶部「组卷」可从题目详情选题、排序和送排。草稿仅保存在浏览器，提交排版不会新增或修改题库内容。

## 目录结构

```text
frontend/
  public/                  图标与工作台插画
  src/main.ts              Angular 启动入口
  src/app/layout/          全局布局与导航
  src/app/core/api/        HTTP 客户端和接口契约
  src/app/features/        题目查询页面
  src/app/shared/latex/    可复用的 KaTeX 渲染组件
backend/
  src/main.rs              服务启动与装配
  src/config.rs            环境变量读取
  src/db.rs                PostgreSQL 连接池、迁移与探活
  src/http/                Axum 路由装配、鉴权和错误映射
  src/http/questions/      题目读写、解答和套题的请求处理
  src/http/export.rs       取题、取图并协调单题导出
  src/questions/           题目数据结构、校验和数据库查询
  src/images.rs            图片校验、命名和存取
  src/storage.rs           通用文件存储接口与本地实现
  src/tex.rs               文档生成、图片引用解析和 ZIP 打包
  src/render_jobs/         组卷快照、任务持久化与独立排版进程
  render/ccho/             固定版 CChO 主题和隔离编译镜像
  migrations/              PostgreSQL 表结构迁移
compose.yaml               本地 PostgreSQL 服务
cli/                       Rust 命令行工具与使用文档
.env.example               本地环境配置示例
```

后端在一个进程内运行。公开路由和管理路由分别组装，仅管理路由统一挂载鉴权中间件。业务 SQL 和事务留在题库模块；文件存储按安全文件键提供限量读取和不覆盖写入，图片格式校验不放进存储层。

## 本地启动（PowerShell）

需要 Node.js 24 LTS、Rust/Cargo（已使用 Rust 1.98 验证）；数据库可用 Docker Compose 启动，也可以连接已有的 PostgreSQL。`Cargo.lock` 与 `frontend/package-lock.json` 固定依赖版本。

**1. 启动数据库。** 在项目根目录执行：

```powershell
Copy-Item .env.example .env
docker compose up -d --wait db
```

已有 PostgreSQL 时，在 `.env` 中填写实际的 `DATABASE_URL`，无需启动容器。后端启动时会自动执行迁移，数据库账号需要建表权限。示例凭据仅用于本地开发；修改数据库名或账号密码时，也要同步修改连接串。不配置 `DATABASE_URL` 可打开页面，但无法查询或新增题目。

**2. 启动后端。** 在项目根目录打开一个终端：

```powershell
cargo run --locked --manifest-path backend/Cargo.toml
```

**3. 启动前端。** 在另一个终端执行：

```powershell
Set-Location frontend
npm.cmd ci
npm.cmd start
```

在浏览器打开 `http://127.0.0.1:4200`。Angular 开发服务器使用 `frontend/proxy.conf.json` 将 `/api/**` 转发至 `127.0.0.1:3000`。如修改 `API_ADDR`，本地代理目标也需同步修改。生产部署时由反向代理将同源 `/api/` 转给后端，并将前端路径（如 `/papers`）回退到 `index.html`；不要使用开发服务器部署。

## 本地组卷与排版

1. 在题目详情选择「加入组卷」，或在套题页点击「整套加入组卷」。整套按原题顺序加入，已有同题合并到原卷顺序，不覆盖其他题或已有卷名；空白卷名自动使用来源名。进入「组卷」可继续调整，每卷最多 40 道。整套超限时不会部分加入。草稿和最近 10 个任务 ID 保存在同一浏览器。
2. 点「送排」后服务器一次读取题目、默认解答及题图引用，固定该次快照。参考答案沿用当前默认解答；缺个别答案时显示「未录入答案」，整卷无解答时只生成题目版。
3. 不需要登录或管理 Key。任务 ID 即取件凭证；不要公开分享。没有服务器端任务列表，结果在完成后可下载 24 小时。

排版默认关闭，不影响查询和本地草稿。启用时使用支持 Linux 容器的 Docker，构建本地镜像：

```powershell
docker build -t shit-pool-ccho:1 backend/render/ccho
```

在根目录的 `.env` 中设置 `SHIT_POOL_RENDER_ENABLED=true` 和可用的 `DATABASE_URL`，按原方式启动 API（自动应用迁移），然后另开终端启动执行器：

```powershell
cargo run --locked --manifest-path backend/Cargo.toml -- render-worker
```

API 与执行器使用同一数据库、配图目录和 `SHIT_POOL_RENDER_DIR`（默认 `data/render`），后者不要设为公开静态目录。排版使用复制进项目的 CChO 主题，不读取个人目录、不回退到宿主机编译。运行要求本地镜像；从任务队列领任务后才启动短时容器，容器不联网、不挂载宿主机目录、不接收服务凭据。开放公网前应额外配置反向代理限流，取件 URL 不写访问日志；当前全站最多同时保留 12 个待处理任务，滚动 24 小时最多接受 60 个任务。执行器会处理过期任务和产物；不要在生产使用内存库。

| 接口 | 行为 |
| --- | --- |
| `POST /api/render-jobs` | 仅接受 `{"title":"周练","question_ids":[8,3],"template":"ccho"}`，成功返回 202 及任务 ID。 |
| `GET /api/render-jobs/{id}` | 返回 `id`、`status`（`queued`、`running`、`completed` 或 `failed`）、`has_answers` 和 `error`。 |
| `GET /api/render-jobs/{id}/files/{kind}` | `kind` 为 `questions` 或 `answers`，任务完成且该文件存在时返回 PDF。 |

## 题库 API

- `GET /api/categories`：公开列出分类 ID、名称和题目数。
- `GET /api/tags`：公开列出标签名称和题目数。
- `GET /api/papers`：公开列出套题来源和已收录题数。
- `GET /api/papers/questions?source=…`：按上一个接口返回的来源读取整套题。原题号升序；同号按 ID 升序；未标记题号的排在最后，按 ID 升序。
- `GET /api/questions`：可选参数为 `q`（题干或来源的字面关键词，不区分大小写）、`category_id`、`tags`（逗号分隔的完整标签名）、`tag_match`（`any` 或 `all`，默认 `any`）和 `page`（从 1 开始，默认 1）。返回 `items`、`page`、`page_size`（10）和筛选后的 `total`。超出最后一页时返回空列表，保留实际总数。
- `GET /api/questions/{id}`：公开读取题目及 `solutions`（每项含 `id`、`title`、`body`、`is_default`，默认解答排在最前）。不存在或软删除返回 404。
- `POST /api/images`：需要同一 Key，上传 PNG 或 JPEG 原始字节（非 multipart）；返回 `name`、`path` 和 `tex`。
- `GET /api/images/{name}`：公开读取已上传图片。
- `GET /api/questions/{id}/export`：公开下载含全部解答及配图的题目 TeX ZIP，缺失或不支持的图片引用返回 422。
- `POST /api/questions`：新增，需要请求头 `Authorization: Bearer <KEY>`；缺少或错误 Key 返回 401，成功返回 201 和新题 ID。
- `PATCH /api/questions/{id}`：需要同一 Key，局部修改；只传需要更新的字段，`notes` 和 `tags` 可用空字符串清空。
- `DELETE /api/questions/{id}`：需要同一 Key，软删除；题目从检索、详情和计数中隐藏，数据库保留记录。
- `POST /api/questions/{id}/solutions`：需要同一 Key，新增解答；JSON 为 `{"title":"另一种解法","body":"LaTeX 正文"}`，可以加 `"make_default":true`。
- `PATCH /api/questions/{id}/solutions/{solution_id}`：需要同一 Key，局部修改 `title` 或 `body`，或传 `"make_default":true` 将该解答设为默认。
- `DELETE /api/questions/{id}/solutions/{solution_id}`：需要同一 Key，软删除该解答。解答写操作返回 `{"question_id":42,"id":17}`，其中 `id` 是解答 ID。
- `GET /api/health`：后端运行与数据库状态检查，不在网页展示。

新增请求使用 `application/json`，示例：

~~~json
{
  "category": "数学",
  "source": "代数练习",
  "stem": "求解 $2x+1=5$。",
  "answer": "移项后得 \\(2x=4\\)，所以 $x=2$。",
  "notes": "一次方程入门",
  "tags": "代数,一次方程"
}
~~~

每题属于一个分类，新增时按名称复用已有分类，不存在则创建。`category`、`source`、`stem` 必填；`answer`、`notes`、`tags` 可省略。标签接受中英文逗号，去除空项和重复后，以英文逗号存入题目表。

每题可有零到多种解答。标题是纯文本（最多 120 字），正文是非空 LaTeX 片段（最多 8000 字）。旧题的非空答案由迁移保留为默认解答。旧 `answer` 字段仍可读写，仅代表默认解答；传空字符串只移除当前默认解答，不影响其他解答。没有显式默认项时，使用最早添加且未删除的解答。查题和套题列表只带兼容字段 `answer`，详情才返回全部解答。

录入套题时，同一试卷的 `source` 填相同的试卷名，题号另填正整数 `original_number`，例如 `{"source":"1992年决赛（理论试题一）","original_number":6}`。不同套别的来源名须保留区别。对旧数据，形如「试卷名第 6 题 · 题名」的来源自动取「试卷名」分组并识别题号，原始来源保留。显式题号优先；修改时用 `"original_number":null` 清除显式题号，恢复对来源的推断。`GET /api/questions` 与按 ID 查询均返回归组后的 `paper_source` 和实际用于排序的 `original_number`。

临时 Key 是 `backend/src/http/auth.rs` 中的 `ADMIN_KEY`，只在服务端校验，不进前端构建。此方案仅供本地开发；外网部署前必须替换硬编码认证并启用 HTTPS。

## 命令行

通过 `cargo install --locked --path cli` 安装 `shit-pool`。支持按 ID 查询、组合检索、TeX 输出以及带 Key 的新增、修改、软删除。命令和输入输出协议见 [CLI 文档](cli/README.md)，Agent 使用约定见 [AGENTS.md](AGENTS.md)。

## LaTeX 渲染

使用随前端打包的 [KaTeX](https://katex.org/docs/options.html)，不依赖外部 CDN。支持 `$...$`、`$$...$$`、`\(...\)`、`\[...\]` 和常用的 `equation`、`align`、`gather` 环境。化学式和物理量支持 `\ce{}` 和 `\pu{}`。TikZ 在网页上仍显示源码，不执行编译。

### 图片上传与 ZIP 导出

运行 `shit-pool upload-image --file figure.png`，把返回的 `tex` 片段放入题干或答案。网页会显示形如 `\includegraphics[width=.5\linewidth]{images/<文件名>}` 的图片；只加载同站已上传的 PNG/JPEG，不加载外链或本地路径。网页不执行 TeX，支持常见的相对宽度；其他排版选项保留在导出的 TeX 中。用宏动态拼路径等完整 TeX 行为不在图片识别范围内。

后端把图片按内容哈希存到 `SHIT_POOL_IMAGE_DIR`，默认是启动目录下的 `data/images`；建议部署时设置绝对路径并与数据库一同备份。相同图片只保存一次；图片是公共资源，软删除题目不会删除图片文件。

详情页「下载 TeX ZIP」或 `shit-pool export 123 --output question-123.zip` 会打包 UTF-8 源文档及引用的图片。解压后进入包含 `images/` 的目录，使用 `xelatex -no-shell-escape question-123.tex` 编译。模板包含 `ctexart`、Fandol、AMS 数学宏包、`graphicx`、TikZ 和 `mhchem`。未声明的其他宏包或外部资源需自行补齐；不要编译来源不明的 TeX。

## 验证

```powershell
cargo fmt --manifest-path backend/Cargo.toml --check
cargo test --locked --manifest-path backend/Cargo.toml
cargo test --locked --manifest-path cli/Cargo.toml
npm.cmd --prefix frontend run build
```
