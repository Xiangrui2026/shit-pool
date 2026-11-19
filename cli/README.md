# SHIT-Pool CLI

从仓库根目录安装：`cargo install --locked --path cli`。不安装也可运行 `cargo run --quiet --locked --manifest-path cli/Cargo.toml -- --help`。

## 连接与认证

地址按 `--url`、`SHIT_POOL_URL`、`http://127.0.0.1:3000` 的优先顺序读取。公网或局域网的写操作要求 HTTPS。只有写操作读取 `SHIT_POOL_API_KEY`；请通过调用环境注入，不接受 Key 参数或配置文件。不会跟随重定向或自动重试。

## 用例

~~~powershell
shit-pool health
shit-pool get 42
shit-pool search -q "期末考试" --page 2
shit-pool categories
shit-pool tags
shit-pool search --category-id 2 --tag 代数 --tag 方程 --tag-match all
shit-pool tex 42 > question-42.tex
shit-pool upload-image --file figure.png
shit-pool export 42 --output question-42.zip
shit-pool add --file question.json
shit-pool update 42 --file patch.json
shit-pool solution add 42 --file solution.json
shit-pool solution update 42 17 --file solution-patch.json
shit-pool solution delete 42 17 --yes
shit-pool delete 42 --yes
~~~

上例的 TeX 重定向适用于 PowerShell 7.4+，它会保留原生命令输出的 UTF-8 字节。`tex` 不执行编译。结果使用同一套题目 ID；`search` 查询题干、来源，不搜索备注，分类与标签条件同时生效。标签可重复传 `--tag`，选择 `any`（默认）或 `all`，精确匹配完整标签名。`search` 返回 `items`、`page`、`page_size`（10）、`total`。

`upload-image` 需要 `SHIT_POOL_API_KEY`。文件支持 PNG 和 JPEG，最大 8 MiB 且宽高各不超过 4096 像素。命令返回 `name`、`path`、`tex`；把返回的 `tex` 片段放入题干或答案，再用 `add` 或 `update` 保存。上传后的图片可公开读取。

`export` 无需 Key，把 `question-42.tex` 和所有引用的图片写进同一个 ZIP，不覆盖已有文件。成功时 stdout 输出 `id`、`output` 和 `bytes`。解压后从包含 `images/` 的目录用 XeLaTeX 编译。ZIP 最多包含 32 张不同图片，图片总计最多 32 MiB。`tex` 只输出源代码，不附带图片；两种导出都包含全部解答。

`question.json`：

~~~json
{
  "category": "数学",
  "source": "代数练习",
  "stem": "计算 \\(1+1\\)。",
  "answer": "$2$",
  "notes": "基础题",
  "tags": "代数,计算"
}
~~~

新增必填 `category`、`source`、`stem`。答案、备注和标签可省略；标签字段是逗号分隔的字符串。不存在的分类名会自动创建。`patch.json` 示例：`{"notes":"","source":"修订版代数练习"}`。未提供的字段保持原值；`notes` 和 `tags` 可传空字符串清空，`answer` 传空字符串只移除当前默认解答。两种写入都可用 `--file -` 读取 UTF-8 stdin。软删除保留数据库记录，但该 ID 无法再通过公开接口读取或更新。删除不会交互询问，必须显式传 `--yes`。

### 多种解答

`get 42` 返回 `solutions` 数组，每项包含解答自己的 `id`、`title`、`body` 和 `is_default`。旧字段 `answer` 只代表默认解答。用 `solution add` 添加解答（例如 `{"title":"代入法","body":"代入得 $x=2$。"}`），用 `solution update` 传局部字段，或传 `{"make_default":true}` 设置默认。新增也可指定 `make_default`。三个解答写入命令都需要 Key，并返回 `{"question_id":42,"id":17}`；更新和删除前先用 `get` 核对解答 ID。新增和修改同样支持 `--file -`。

## 输出约定

录入套题时，同一试卷使用相同的 `source`，并在新增或局部修改 JSON 中通过 `original_number` 填写正整数题号。题号可省略；修改时可用 `null` 清除显式编号，继续从来源中的「第 N 题」识别。读取结果包含 `paper_source` 和实际用于排序的 `original_number`。

- 成功：stdout 一个 JSON 值并以换行结束。`get` 返回题目对象，写操作返回 `{"id":42}`。`tex` 例外，只输出原始 TeX；`--help`、`--version` 输出普通文本。
- 失败：stdout 为空（本地输出失败除外），stderr 输出 `{"error":"not_found","message":"question_not_found","status":404}` 形态的 JSON。`status` 仅在收到 HTTP 错误时存在。不要用文字解析代替退出码。

| 退出码 | 含义 |
| --- | --- |
| 0 | 成功，包括查询无结果 |
| 2 | 参数、输入或请求校验失败 |
| 3 | 缺少 Key 或鉴权失败 |
| 4 | 题目或目标接口不存在 |
| 5 | 连接失败、超时、服务端错误或响应格式错误 |
| 6 | 本地文件或输出 I/O 失败 |
