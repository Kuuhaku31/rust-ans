
// src/main.rs
// 递归解析 Rust 函数定义并导出为 CSV

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{ImplItem, Item, TraitItem};

/// 表示一个函数定义及其源码位置
struct FunctionRecord {
    /// 函数所在源码文件的相对路径
    file_path: String,
    /// 函数在 crate 模块结构中的完整名称
    full_name: String,
    /// 函数定义的起始行号
    start_line: usize,
    /// 函数定义的结束行号
    end_line: usize,
}

/// 从命令行读取目录和输出路径并完成索引
fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != 3 {
        return Err("用法: rust-ans <Rust 源码目录> <输出 CSV 文件>".into());
    }

    let source_dir = Path::new(&arguments[1]);
    let output_path = Path::new(&arguments[2]);
    if !source_dir.is_dir() {
        return Err(format!("源码目录不存在或不是目录: {}", source_dir.display()).into());
    }

    let mut source_files = Vec::new();
    collect_rust_files(source_dir, &mut source_files)?;
    source_files.sort();

    let mut records = Vec::new();
    for source_file in source_files {
        let relative_path = source_file.strip_prefix(source_dir)?;
        let relative_path = relative_path.to_string_lossy().replace('\\', "/");
        let module_path = module_path_for_file(&source_file, source_dir)?;
        let source = fs::read_to_string(&source_file)?;
        let syntax = syn::parse_file(&source)?;
        collect_items(&syntax.items, &module_path, &relative_path, &mut records);
    }

    records.sort_by(|left, right| {
        left.file_path
            .cmp(&right.file_path)
            .then(left.start_line.cmp(&right.start_line))
    });

    let mut csv = String::from("文件相对路径,函数完整名称,起始行,结束行\n");
    for record in records {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            csv_field(&record.file_path),
            csv_field(&record.full_name),
            record.start_line,
            record.end_line
        ));
    }
    fs::write(output_path, csv)?;
    Ok(())
}

/// 递归收集目录中的 Rust 源文件
fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// 根据 Rust 源文件路径生成对应的 crate 模块路径
fn module_path_for_file(file: &Path, source_dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let relative_path = file.strip_prefix(source_dir)?;
    let components: Vec<String> = relative_path
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(str::to_owned))
        .collect();
    let mut modules = Vec::new();

    for (index, component) in components.iter().enumerate() {
        let is_last = index + 1 == components.len();
        if is_last {
            let stem = Path::new(component)
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("无效的 Rust 源文件路径: {}", file.display()))?;
            if stem != "main" && stem != "lib" && stem != "mod" {
                modules.push(stem.to_owned());
            }
        } else if index != 0 || component != "src" {
            modules.push(component.clone());
        }
    }
    Ok(modules)
}

/// 遍历模块级语法项并收集函数, 方法和内联模块中的函数
fn collect_items(
    items: &[Item],
    module_path: &[String],
    file_path: &str,
    records: &mut Vec<FunctionRecord>,
) {
    for item in items {
        match item {
            Item::Fn(function) => records.push(make_record(
                function.sig.ident.to_string(),
                function.span(),
                module_path,
                file_path,
            )),
            Item::Impl(implementation) => {
                let type_name = type_path_name(&implementation.self_ty);
                let mut method_path = module_path.to_vec();
                method_path.push(type_name);
                for item in &implementation.items {
                    if let ImplItem::Fn(method) = item {
                        records.push(make_record(
                            method.sig.ident.to_string(),
                            method.span(),
                            &method_path,
                            file_path,
                        ));
                    }
                }
            }
            Item::Trait(trait_item) => {
                let mut trait_path = module_path.to_vec();
                trait_path.push(trait_item.ident.to_string());
                for item in &trait_item.items {
                    if let TraitItem::Fn(method) = item {
                        records.push(make_record(
                            method.sig.ident.to_string(),
                            method.span(),
                            &trait_path,
                            file_path,
                        ));
                    }
                }
            }
            Item::Mod(module) => {
                if let Some((_, nested_items)) = &module.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(module.ident.to_string());
                    collect_items(nested_items, &nested_path, file_path, records);
                }
            }
            _ => {}
        }
    }
}

/// 根据函数名称, 源码范围和模块路径构造索引记录
fn make_record(
    name: String,
    span: Span,
    module_path: &[String],
    file_path: &str,
) -> FunctionRecord {
    let full_name = if module_path.is_empty() {
        format!("crate::{}()", name)
    } else {
        format!("crate::{}::{}()", module_path.join("::"), name)
    };
    let start = span.start();
    let end = span.end();
    FunctionRecord {
        file_path: file_path.to_owned(),
        full_name,
        start_line: start.line,
        end_line: end.line,
    }
}

/// 获取 impl 类型路径的名称, 用于组成方法完整名称
fn type_path_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        _ => "impl".to_owned(),
    }
}

/// 按 CSV 规则转义字段中的逗号, 引号和换行符
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
