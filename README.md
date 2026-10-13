# Rust 函数定义索引工具需求说明

## 功能目标

递归扫描指定的 Rust 源码目录, 提取所有函数定义及其源码位置,
将结果导出为 CSV 文件, 供后续函数调用分析, 代码导航和调用流程图生成使用.

## 使用方式

传入 Rust 源码目录和 CSV 输出路径:

```sh
cargo run --bin rust-ans -- ./src ./functions.csv
```

- **输入:** 一个 Rust 源码目录, 例如 `./src/`.
- **输出:** 一个 CSV 文件, 例如 `./functions.csv`.

## draw.io 图形生成

将函数索引 CSV 转换为 draw.io 可导入的 mxGraphModel XML:

```sh
cargo run --bin csv_to_drawio -- ./functions.csv ./functions.xml
```

可选参数用于调整容器网格、尺寸和函数矩形布局:

```sh
cargo run --bin csv_to_drawio -- --help
```

## XML 与 URL 编码互转

自动识别 XML 或 URL 编码的 XML 并转换. 默认输出文件后缀分别为 `.txt` 和 `.xml`:

```sh
cargo run --bin to_xml -- ./functions.xml
cargo run --bin to_xml -- ./functions.txt --output ./functions-decoded.xml
```

```sh
cargo run --bin to_xml -- --help
```

## 扫描规则

递归扫描输入目录下的所有 `.rs` 文件, 提取以下函数定义:

- 普通函数, 包括异步函数.
- `impl` 块中的方法.
- `trait` 中声明或定义的方法.
- 内联模块中的函数.
- 外部模块文件中的函数, 包括 `mod.rs`, `foo.rs` 和 `foo/` 目录下的模块文件.

函数范围以 Rust 语法解析结果为准, 不使用正则表达式推断函数边界.

- 不包括函数体内部定义的嵌套函数.
- 不要求分析宏展开后生成的函数.
- 注释, 字符串中的函数声明文本以及函数调用表达式均不作为函数定义记录.

## CSV 格式

CSV 包含以下六列, 列名和顺序固定:

| 列名         | 含义                             | 示例                   |
| ------------ | -------------------------------- | ---------------------- |
| 文件相对路径 | `.rs` 文件相对于输入目录的路径   | `models.rs`            |
| 函数完整名称 | 函数在 Rust 模块结构中的完整名称 | `crate::models::fun()` |
| 可见性       | Rust 函数的可见性                | `pub(crate)`           |
| 起始行       | 函数定义的起始行号               | `12`                   |
| 结束行       | 函数定义的结束行号               | `25`                   |
| 行数         | 函数定义覆盖的总行数             | `14`                   |

行号从 `1` 开始, 起始行和结束行均包含在内.
未显式声明可见性的函数记录为 `private`. draw.io 函数矩形显示可见性; `pub(crate)` 函数使用随源码行数递增加深的绿色系填充色, 其他函数使用蓝色系填充色.

- 按文件相对路径排序, 同一文件中的函数按起始行升序排列.
