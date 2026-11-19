# CChO 试卷排版

`profile.json` 与 `paper-core.sty` 分别复制自 LaTeX 排版系统的 `styles/ccho/profile.json` 和 `assets/paper-core.sty`。这是 CChO 试卷风格，不是同名的空白答题卡。运行时不读取原始目录；更新主题时一并核对两个文件。

石库根据题目快照构建共用的 `paper.tex`。题目和答案各在独立容器中编译，题目版容器不会收到解答正文。仅在答案版提供现有默认解答，不生成或验证学科答案。

根目录运行 `docker build -t shit-pool-ccho:1 backend/render/ccho` 构建隔离执行镜像。它只接受执行器生成的输入，不是通用的公开 TeX 编译服务。上线时可通过 `SHIT_POOL_RENDER_IMAGE` 指向本地已审核的镜像标签或摘要。

本机安装 XeLaTeX 与 Poppler 时，可运行 `cargo test --locked --manifest-path backend/Cargo.toml compiles_and_checks_both_ccho_pdfs -- --ignored` 验证固定合成样例。此测试不读取题库，也不替代隔离容器的部署验证。
