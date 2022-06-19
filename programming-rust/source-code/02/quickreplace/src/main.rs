use regex::Regex;
use std::env;
use std::fs;
use text_colorizer::*;

#[derive(Debug)]
struct Arguments {
    target: String,
    replacement: String,
    filename: String,
    output: String,
}

fn main() {
    let args = parse_args();

    let data = match fs::read_to_string(&args.filename) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "{} 读取文件'{}'失败: {:?}",
                "错误:".red().bold(),
                args.filename,
                e
            );
            std::process::exit(1);
        }
    };

    let _replace_data = match replace(&args.target, &args.replacement, &data) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} 替换文本失败: {:?}", "错误:".red().bold(), e);
            std::process::exit(1);
        }
    };

    match fs::write(&args.output, &data) {
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "{} 读取文件'{}'失败: {:?}",
                "错误:".red().bold(),
                args.filename,
                e
            );
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!(
        "{} - 将一个字符串替换为另一个字符串",
        "quickreplace".green()
    );
    eprintln!("Usage: quickreplace <target> <replacement> <INPUT> <OUTPUT>");
}

fn parse_args() -> Arguments {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() != 4 {
        print_usage();
        eprintln!(
            "{} 参数数量不符：需要4个参数，传入了{}个。",
            "错误:".red().bold(),
            args.len()
        );
        std::process::exit(1);
    }

    Arguments {
        target: args[0].clone(),
        replacement: args[1].clone(),
        filename: args[2].clone(),
        output: args[3].clone(),
    }
}

fn replace(target: &str, replacement: &str, text: &str) -> Result<String, regex::Error> {
    let regex = Regex::new(target)?;
    Ok(regex.replace_all(text, replacement).to_string())
}
