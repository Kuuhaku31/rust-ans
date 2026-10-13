
// src/bin/to_xml.rs
// 自动识别并双向转换普通 XML 与 URL 编码 XML。

use std::error::Error;
use std::path::PathBuf;

/// 判断文本是否为有效 XML。
fn is_xml(text: &str) -> bool {
    roxmltree::Document::parse(text.trim()).is_ok()
}

/// 将 URL 编码文本解码为 UTF-8 字符串。
fn url_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[index + 1]), hex_value(bytes[index + 2])) {
                decoded.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// 将一个十六进制字节转换为对应数值。
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// 将文本编码为百分号形式并保留 RFC 3986 非保留字符。
fn url_encode(text: &str) -> String {
    let mut encoded = String::new();
    for byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// 自动识别 XML 输入格式并返回转换结果和模式说明。
fn convert(text: &str) -> Result<(String, &'static str), Box<dyn Error>> {
    let text = text.trim();
    if is_xml(text) {
        return Ok((url_encode(text), "XML -> URL 编码"));
    }
    let decoded = url_decode(text);
    if decoded != text && is_xml(&decoded) {
        return Ok((decoded, "URL 编码 -> XML"));
    }
    Err("无法识别输入格式: 内容既不是有效 XML, 也不是 URL 编码的 XML.".into())
}

/// 显示命令帮助并解析输入及可选输出路径。
fn parse_args() -> Result<(PathBuf, Option<PathBuf>), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut input = None;
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                println!("用法: to_xml <输入文件> [-o|--output <输出文件>]\n自动识别并双向转换 XML 与 URL 编码.");
                std::process::exit(0);
            }
            "-o" | "--output" => {
                if output.is_some() {
                    return Err("输出路径不能重复指定.".into());
                }
                output = Some(PathBuf::from(
                    args.next().ok_or("选项 --output 缺少文件路径.")?,
                ));
            }
            _ if argument.starts_with('-') => {
                return Err(format!("未知选项: {argument}.").into());
            }
            _ => {
                if input.replace(PathBuf::from(argument)).is_some() {
                    return Err("只能指定一个输入文件.".into());
                }
            }
        }
    }
    let input = input.ok_or("用法: to_xml <输入文件> [-o|--output <输出文件>].")?;
    Ok((input, output))
}

/// 读取输入文件, 转换格式并写入选定的输出文件。
fn run() -> Result<(), Box<dyn Error>> {
    let (input_path, output_path) = parse_args()?;
    let text = std::fs::read_to_string(&input_path)?;
    let (result, mode) = convert(&text)?;
    let output_path = output_path.unwrap_or_else(|| {
        input_path.with_extension(if mode == "XML -> URL 编码" {
            "txt"
        } else {
            "xml"
        })
    });
    std::fs::write(&output_path, result)?;
    println!("转换类型: {mode}");
    println!("输出文件: {}", output_path.display());
    Ok(())
}

/// 执行命令并以非零状态报告转换错误。
fn main() {
    if let Err(error) = run() {
        eprintln!("错误: {error}");
        std::process::exit(1);
    }
}
