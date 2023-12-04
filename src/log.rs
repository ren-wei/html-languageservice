use std::fs::OpenOptions;
use std::io::Write;

pub fn log(msg: &str) {
    let file_path = "/Users/renwei/Code/html-languageservice/log.txt";

    // 打开文件并追加内容
    let mut file = OpenOptions::new().append(true).open(file_path).unwrap();

    // 写入内容到文件末尾
    writeln!(file, "{}", msg).unwrap();
}
