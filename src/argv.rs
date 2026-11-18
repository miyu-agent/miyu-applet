//! 测试用：照 Windows 的 `CommandLineToArgvW` 把一行命令拆成参数（启动项、网址协议登记的那一行，系统就是这么拆的）。只在测试里编，
//! 哪个系统上都能跑。
//!
//! 规矩（微软文档「Parsing C++ command-line arguments」）：第一个（程序）从引号到下一个引号、不认反斜杠；后面的空白分隔，
//! 引号里的空白不算；`2n` 个反斜杠跟着引号是 `n` 个反斜杠、引号开关一次，`2n+1` 个是 `n` 个反斜杠加一个字面的引号；不跟着引号的反斜杠照原样；
//! 引号里连着两个引号是一个字面的引号。

/// 拆成参数。
pub fn split(line: &str) -> Vec<String> {
    let mut chars = line.chars().peekable();
    let mut args = Vec::new();
    // 程序：引号开头的到下一个引号，不然到空白。
    let mut program = String::new();
    if chars.peek() == Some(&'"') {
        chars.next();
        for c in chars.by_ref() {
            if c == '"' {
                break;
            }
            program.push(c);
        }
    }
    while let Some(&c) = chars.peek() {
        if c == ' ' || c == '\t' {
            break;
        }
        program.push(c);
        chars.next();
    }
    args.push(program);
    loop {
        while chars.peek().is_some_and(|c| *c == ' ' || *c == '\t') {
            chars.next();
        }
        if chars.peek().is_none() {
            return args;
        }
        let mut arg = String::new();
        let mut quoted = false;
        while let Some(&c) = chars.peek() {
            match c {
                '\\' => {
                    let mut slashes = 0;
                    while chars.peek() == Some(&'\\') {
                        chars.next();
                        slashes += 1;
                    }
                    if chars.peek() == Some(&'"') {
                        arg.extend(std::iter::repeat_n('\\', slashes / 2));
                        if slashes % 2 == 1 {
                            arg.push('"');
                            chars.next();
                        }
                    } else {
                        arg.extend(std::iter::repeat_n('\\', slashes));
                    }
                }
                '"' => {
                    chars.next();
                    if quoted && chars.peek() == Some(&'"') {
                        arg.push('"');
                        chars.next();
                    } else {
                        quoted = !quoted;
                    }
                }
                ' ' | '\t' if !quoted => break,
                other => {
                    arg.push(other);
                    chars.next();
                }
            }
        }
        args.push(arg);
    }
}

#[test]
fn the_rules_as_documented() {
    // 微软文档里的例子。
    assert_eq!(split(r#"p "abc" d e"#), ["p", "abc", "d", "e"]);
    assert_eq!(split(r#"p a\\\b d"e f"g h"#), ["p", r"a\\\b", "de fg", "h"]);
    assert_eq!(split(r#"p a\\\"b c d"#), ["p", r#"a\"b"#, "c", "d"]);
    assert_eq!(split(r#"p a\\\\"b c" d e"#), ["p", r"a\\b c", "d", "e"]);
    assert_eq!(
        split(r#""C:\Program Files\x.exe" --home "D:\m""#),
        [r"C:\Program Files\x.exe", "--home", r"D:\m"]
    );
}
