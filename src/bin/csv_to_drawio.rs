
// src/bin/csv_to_drawio.rs
// 将函数索引 CSV 转换为 draw.io mxGraphModel XML。

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

const FILE_COLUMN: &str = "文件相对路径";
const NAME_COLUMN: &str = "函数完整名称";
const VISIBILITY_COLUMN: &str = "可见性";
const START_COLUMN: &str = "起始行";
const END_COLUMN: &str = "结束行";

/// 表示 CSV 中的单个函数及其源码范围。
struct FunctionRecord {
    /// 函数完整名称。
    name: String,
    /// 函数的 Rust 可见性。
    visibility: String,
    /// 函数定义的起始行号。
    start_line: usize,
    /// 函数定义的结束行号。
    end_line: usize,
}

/// 保存文件容器和容器内函数的布局数据。
struct FileLayout {
    /// 文件相对路径。
    path: String,
    /// 文件中按起始行排序的函数列表。
    functions: Vec<FunctionRecord>,
    /// 每个函数矩形的高度。
    function_heights: Vec<usize>,
    /// 文件容器的总高度。
    height: usize,
    /// 文件容器的横向位置。
    x: usize,
    /// 文件容器的纵向位置。
    y: usize,
}

/// 保存命令行布局参数。
struct Options {
    /// CSV 输入路径。
    input_csv: PathBuf,
    /// XML 输出路径。
    output_xml: PathBuf,
    /// 每行容器数量。
    columns: usize,
    /// 容器宽度。
    container_width: usize,
    /// 容器水平间距。
    horizontal_gap: usize,
    /// 容器垂直间距。
    vertical_gap: usize,
    /// 每个源码行对应的高度。
    line_height: usize,
    /// 函数矩形高度的源码行数上限。
    max_height_lines: usize,
    /// 函数矩形最小高度。
    min_function_height: usize,
    /// 函数矩形额外高度。
    function_padding: usize,
    /// 函数名称的换行字符数。
    chars_per_line: usize,
    /// 文件容器标题栏高度。
    header_height: usize,
    /// 容器内部边距。
    container_padding: usize,
    /// 函数矩形之间的间距。
    function_gap: usize,
}

/// 解析命令行参数并返回布局配置。
fn parse_args() -> Result<Options, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut positional = Vec::new();
    let mut options = BTreeMap::new();
    while let Some(argument) = args.next() {
        if argument == "-h" || argument == "--help" {
            println!("用法: csv_to_drawio <输入 CSV> <输出 XML> [选项]\n\
选项:\n\
  --columns <整数>             每行容器数量, 默认 30\n\
  --container-width <整数>     文件容器宽度, 默认 360\n\
  --horizontal-gap <整数>      容器水平间距, 默认 48\n\
  --vertical-gap <整数>        容器垂直间距, 默认 48\n\
  --line-height <整数>         每个源码行对应的高度, 默认 14\n\
  --max-height-lines <整数>    函数矩形高度上限行数, 默认 12\n\
  --min-function-height <整数> 函数矩形最小高度, 默认 52\n\
  --function-padding <整数>    函数矩形额外高度, 默认 20\n\
  --chars-per-line <整数>      函数名称换行字符数, 默认 40\n\
  --header-height <整数>       容器标题栏高度, 默认 32\n\
  --container-padding <整数>   容器内部边距, 默认 12\n\
  --function-gap <整数>        函数矩形之间的间距, 默认 8");
            std::process::exit(0);
        } else if argument.starts_with("--") {
            let key = argument.trim_start_matches("--").to_owned();
            let value = args.next().ok_or_else(|| format!("选项 --{key} 缺少数值."))?;
            if options.insert(key.clone(), value).is_some() {
                return Err(format!("选项 --{key} 重复指定.").into());
            }
        } else {
            positional.push(argument);
        }
    }
    if positional.len() != 2 {
        return Err("用法: csv_to_drawio <输入 CSV> <输出 XML> [选项].".into());
    }

    let mut result = Options {
        input_csv: PathBuf::from(&positional[0]),
        output_xml: PathBuf::from(&positional[1]),
        columns: 30,
        container_width: 360,
        horizontal_gap: 48,
        vertical_gap: 48,
        line_height: 14,
        max_height_lines: 12,
        min_function_height: 52,
        function_padding: 20,
        chars_per_line: 40,
        header_height: 32,
        container_padding: 12,
        function_gap: 8,
    };

    for (key, value) in options {
        let number = value
            .parse::<i64>()
            .map_err(|_| format!("选项 --{key} 必须是整数."))?;
        let target = match key.as_str() {
            "columns" => &mut result.columns,
            "container-width" => &mut result.container_width,
            "horizontal-gap" => &mut result.horizontal_gap,
            "vertical-gap" => &mut result.vertical_gap,
            "line-height" => &mut result.line_height,
            "max-height-lines" => &mut result.max_height_lines,
            "min-function-height" => &mut result.min_function_height,
            "function-padding" => &mut result.function_padding,
            "chars-per-line" => &mut result.chars_per_line,
            "header-height" => &mut result.header_height,
            "container-padding" => &mut result.container_padding,
            "function-gap" => &mut result.function_gap,
            _ => return Err(format!("未知选项: --{key}.").into()),
        };
        if number < 0 {
            return Err(format!("选项 --{key} 不能为负数.").into());
        }
        *target = number as usize;
    }
    validate_options(&result)?;
    Ok(result)
}

/// 校验布局参数满足生成图形所需的范围。
fn validate_options(options: &Options) -> Result<(), Box<dyn Error>> {
    if options.columns == 0 {
        return Err("--columns 必须大于 0.".into());
    }
    for (name, value) in [
        ("container-width", options.container_width),
        ("line-height", options.line_height),
        ("max-height-lines", options.max_height_lines),
        ("min-function-height", options.min_function_height),
        ("chars-per-line", options.chars_per_line),
        ("header-height", options.header_height),
    ] {
        if value == 0 {
            return Err(format!("--{name} 必须大于 0.").into());
        }
    }
    if options.container_width <= options.container_padding.saturating_mul(2) {
        return Err("容器宽度必须大于两倍容器边距.".into());
    }
    Ok(())
}

/// 读取 CSV 并校验必要列和每行函数记录。
fn read_csv(path: &Path) -> Result<BTreeMap<String, Vec<FunctionRecord>>, Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    let index = |column: &str| {
        headers
            .iter()
            .position(|header| header == column)
            .ok_or_else(|| format!("CSV 缺少必需列: {column}"))
    };
    let file_index = index(FILE_COLUMN)?;
    let name_index = index(NAME_COLUMN)?;
    let visibility_index = headers.iter().position(|header| header == VISIBILITY_COLUMN);
    let start_index = index(START_COLUMN)?;
    let end_index = index(END_COLUMN)?;
    let mut grouped: BTreeMap<String, Vec<FunctionRecord>> = BTreeMap::new();

    for (row_index, row) in reader.records().enumerate() {
        let row = row?;
        let line_number = row_index + 2;
        let file_path = row.get(file_index).unwrap_or("").trim();
        let name = row.get(name_index).unwrap_or("").trim();
        let visibility = visibility_index
            .and_then(|index| row.get(index))
            .unwrap_or("")
            .trim();
        if file_path.is_empty() || name.is_empty() {
            return Err(format!("CSV 第 {line_number} 行缺少文件路径或函数名称.").into());
        }
        let start_line = row
            .get(start_index)
            .unwrap_or("")
            .parse::<usize>()
            .map_err(|_| format!("CSV 第 {line_number} 行的起止行必须为整数."))?;
        let end_line = row
            .get(end_index)
            .unwrap_or("")
            .parse::<usize>()
            .map_err(|_| format!("CSV 第 {line_number} 行的起止行必须为整数."))?;
        if start_line == 0 || end_line < start_line {
            return Err(format!("CSV 第 {line_number} 行的起止行不合法.").into());
        }
        grouped
            .entry(file_path.to_owned())
            .or_default()
            .push(FunctionRecord {
                name: name.to_owned(),
                visibility: visibility.to_owned(),
                start_line,
                end_line,
            });
    }

    for functions in grouped.values_mut() {
        functions.sort_by(|left, right| {
            left.start_line
                .cmp(&right.start_line)
                .then(left.name.cmp(&right.name))
        });
    }
    Ok(grouped)
}

/// 将函数名称按模块分隔符拆分为可显示的多行标签。
fn split_name(name: &str, chars_per_line: usize) -> Vec<String> {
    let parts: Vec<&str> = name.split("::").collect();
    let mut lines = Vec::new();
    let mut current = parts[0].to_owned();
    for part in parts.iter().skip(1) {
        let candidate = format!("{current}::{part}");
        if candidate.chars().count() > chars_per_line && !current.is_empty() {
            lines.push(format!("{current}::"));
            current = (*part).to_owned();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }

    let mut wrapped = Vec::new();
    for line in lines {
        if line.chars().count() <= chars_per_line || line.ends_with("::") {
            wrapped.push(line);
        } else {
            let characters: Vec<char> = line.chars().collect();
            wrapped.extend(
                characters
                    .chunks(chars_per_line)
                    .map(|chunk| chunk.iter().collect()),
            );
        }
    }
    if wrapped.is_empty() {
        wrapped.push(name.to_owned());
    }
    wrapped
}

/// 计算函数名称标签的换行行数。
fn estimate_wrapped_lines(name: &str, chars_per_line: usize) -> usize {
    split_name(name, chars_per_line).len()
}

/// 根据函数行数和标签长度计算文件容器高度。
fn build_layouts(grouped: BTreeMap<String, Vec<FunctionRecord>>, options: &Options) -> Vec<FileLayout> {
    grouped
        .into_iter()
        .map(|(path, functions)| {
            let function_heights: Vec<usize> = functions
                .iter()
                .map(|function| {
                    let line_count = function.end_line - function.start_line + 1;
                    let visible_lines = line_count.min(options.max_height_lines);
                    let source_height = visible_lines
                        .saturating_mul(options.line_height)
                        .saturating_add(options.function_padding);
                    let label_height = estimate_wrapped_lines(&function.name, options.chars_per_line)
                        .saturating_add(1)
                        .saturating_mul(16)
                        .saturating_add(options.function_padding);
                    options
                        .min_function_height
                        .max(source_height)
                        .max(label_height)
                })
                .collect();
            let content_height = if functions.is_empty() {
                options.min_function_height
            } else {
                function_heights.iter().sum::<usize>()
                    + options.function_gap.saturating_mul(functions.len() - 1)
            };
            FileLayout {
                path,
                functions,
                function_heights,
                height: options.header_height
                    + options.container_padding.saturating_mul(2)
                    + content_height,
                x: 0,
                y: 0,
            }
        })
        .collect()
}

/// 按网格排列文件容器并为每一行计算纵向位置。
fn place_containers(layouts: &mut [FileLayout], options: &Options) {
    let mut row_y = 0;
    for row in layouts.chunks_mut(options.columns) {
        let row_height = row.iter().map(|layout| layout.height).max().unwrap_or(0);
        for (column, layout) in row.iter_mut().enumerate() {
            layout.x = column.saturating_mul(options.container_width + options.horizontal_gap);
            layout.y = row_y;
        }
        row_y = row_y.saturating_add(row_height + options.vertical_gap);
    }
}

/// 转义 XML 文本或属性值中的特殊字符。
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 根据函数名称和源码范围生成带换行及行号的标签。
fn function_label(function: &FunctionRecord, chars_per_line: usize) -> String {
    let name = split_name(&function.name, chars_per_line)
        .iter()
        .map(|part| xml_escape(part))
        .collect::<Vec<_>>()
        .join("<br/>");
    let line_count = function.end_line - function.start_line + 1;
    format!(
        "{name}<br/><span style=\"font-size:10px;color:#666666\">可见性: {} | L{}-L{} ({} lines)</span>",
        xml_escape(&function.visibility),
        function.start_line,
        function.end_line,
        line_count
    )
}

/// 根据源码行数和可见性选择函数矩形的填充颜色。
fn function_fill_color(
    line_count: usize,
    max_height_lines: usize,
    visibility: &str,
) -> &'static str {
    // 根据可见性选择颜色渐变, 绿色表示 pub(crate), 蓝色表示其他可见性。
    let colors = if visibility == "pub(crate)" {
        [
            "#F5FAF4", // 正常范围: 极浅绿
            "#E7F2E4", // 1.0-1.5 倍: 浅绿
            "#D3E7CE", // 1.5-2.0 倍: 柔和绿
            "#B9D9B2", // 2.0-3.0 倍: 中浅绿
            "#98C68F", // 3.0-5.0 倍: 中绿
            "#79B36F", // 5.0-8.0 倍: 深绿
            "#5E9656", // 超过 8.0 倍: 深绿
        ]
    } else {
        [
            "#FFFFFF", // 正常范围: 白色
            "#F0F6FD", // 1.0-1.5 倍: 极浅蓝
            "#DDEAF9", // 1.5-2.0 倍: 浅蓝
            "#C4DCF5", // 2.0-3.0 倍: 中浅蓝
            "#A5C9ED", // 3.0-5.0 倍: 中蓝
            "#83B3E2", // 5.0-8.0 倍: 深蓝
            "#6397CD", // 超过 8.0 倍: 深蓝
        ]
    };

    // 避免 max_height_lines 为 0 时发生除零问题。
    let max_height_lines = max_height_lines.max(1);
    let ratio = line_count as f64 / max_height_lines as f64;

    // 根据函数行数占最大高度的比例选择颜色等级。
    let index = if line_count <= max_height_lines {
        0
    } else if ratio <= 1.5 {
        1
    } else if ratio <= 2.0 {
        2
    } else if ratio <= 3.0 {
        3
    } else if ratio <= 5.0 {
        4
    } else if ratio <= 8.0 {
        5
    } else {
        6
    };

    colors[index]
}

/// 将布局数据序列化为 draw.io mxGraphModel XML 字符串。
fn generate_xml(layouts: &[FileLayout], options: &Options) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<mxGraphModel dx=\"1400\" dy=\"900\" grid=\"1\" gridSize=\"10\" guides=\"1\" tooltips=\"1\" connect=\"1\" arrows=\"1\" fold=\"1\" page=\"1\" pageScale=\"1\" pageWidth=\"1169\" pageHeight=\"827\" math=\"0\" shadow=\"0\">\n  <root>\n    <mxCell id=\"0\"/>\n    <mxCell id=\"1\" parent=\"0\"/>\n",
    );
    let container_style = "swimlane;html=1;horizontal=1;startSize=32;rounded=0;collapsible=0;whiteSpace=wrap;fillColor=#f5f5f5;swimlaneFillColor=#ffffff;strokeColor=#666666;fontStyle=1;fontSize=13;";
    let function_style = "rounded=0;whiteSpace=wrap;html=1;overflow=hidden;align=left;verticalAlign=middle;spacingLeft=8;spacingRight=8;strokeColor=#6c8ebf;fontSize=12;";
    let mut next_id = 2;
    for layout in layouts {
        let container_id = next_id;
        next_id += 1;
        xml.push_str(&format!(
            "    <mxCell id=\"{container_id}\" value=\"{}\" style=\"{container_style}\" vertex=\"1\" parent=\"1\">\n      <mxGeometry x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" as=\"geometry\"/>\n    </mxCell>\n",
            xml_escape(&layout.path), layout.x, layout.y, options.container_width, layout.height
        ));
        let mut child_y = options.header_height + options.container_padding;
        let child_width = options.container_width - 2 * options.container_padding;
        for (function, function_height) in layout.functions.iter().zip(&layout.function_heights) {
            let line_count = function.end_line - function.start_line + 1;
            let fill_color = function_fill_color(
                line_count,
                options.max_height_lines,
                &function.visibility,
            );
            xml.push_str(&format!(
                "    <mxCell id=\"{next_id}\" value=\"{}\" style=\"{function_style}fillColor={fill_color};\" vertex=\"1\" parent=\"{container_id}\">\n      <mxGeometry x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" as=\"geometry\"/>\n    </mxCell>\n",
                xml_escape(&function_label(function, options.chars_per_line)),
                options.container_padding,
                child_y,
                child_width,
                function_height
            ));
            next_id += 1;
            child_y += function_height + options.function_gap;
        }
    }
    xml.push_str("  </root>\n</mxGraphModel>\n");
    xml
}

/// 解析 CSV, 生成 draw.io XML 并报告输出结果。
fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_args()?;
    let grouped = read_csv(&options.input_csv)?;
    let file_count = grouped.len();
    let function_count: usize = grouped.values().map(Vec::len).sum();
    let mut layouts = build_layouts(grouped, &options);
    place_containers(&mut layouts, &options);
    if let Some(parent) = options
        .output_xml
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&options.output_xml, generate_xml(&layouts, &options))?;
    println!(
        "已生成 {}: {} 个文件容器, {} 个函数节点.",
        options.output_xml.display(),
        file_count,
        function_count
    );
    Ok(())
}

/// 执行命令并以非零状态报告输入或输出错误。
fn main() {
    if let Err(error) = run() {
        eprintln!("错误: {error}");
        std::process::exit(1);
    }
}
