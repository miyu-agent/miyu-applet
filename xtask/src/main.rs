//! 托盘仓库的门禁程序（照网页仓库的 `xtask`，那一份照 miyu-im 的；施工 AP-5）：`cargo xtask licenses` 查发布的四个平台上用得到的
//! 第三方依赖，许可证都能和 AGPL-3.0-or-later 合在一起发（[`licenses`]），托盘的第三方许可证清单是最新的（[`notices`]，照核心施工
//! AU-2）；`cargo xtask notices` 重新生成那份清单。
//! `cargo xtask privacy` 查被 git 跟踪的文本里没有带真实用户名的路径（[`privacy`]，照核心的同名门禁）。格式、clippy、测试、行数照旧是 CI 里一步一步的 cargo 命令。

mod licenses;
mod notices;
mod privacy;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("licenses") => check_licenses(),
        Some("privacy") => check_privacy(),
        Some("notices") => {
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
            match notices::write(&root(), &cargo) {
                Ok(()) => {
                    println!("写好了 {}", notices::PATH);
                    ExitCode::SUCCESS
                }
                Err(problem) => {
                    eprintln!("许可证清单：{problem}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            eprintln!("用法：cargo xtask licenses | cargo xtask notices | cargo xtask privacy");
            ExitCode::from(2)
        }
    }
}

/// 仓库的根目录。
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask 在仓库根目录下的 xtask/ 里，一定有上一级目录")
        .to_path_buf()
}

/// 跑一遍许可证：都能用、清单是最新的印一行、退出码 0；有问题的一包一行、退出码 1。
fn check_licenses() -> ExitCode {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut problems = licenses::check(&root(), &cargo);
    problems.extend(notices::check(&root(), &cargo));
    if problems.is_empty() {
        println!(
            "许可证：{} 四个平台上的第三方依赖都能和 AGPL-3.0-or-later 合在一起发，{} 是最新的",
            licenses::PLATFORMS.join("、"),
            notices::PATH
        );
        return ExitCode::SUCCESS;
    }
    for problem in &problems {
        eprintln!("许可证：{problem}");
    }
    ExitCode::from(1)
}

/// 跑一遍私密路径：没有违例的印一行、退出码 0；有的一处一行、退出码 1。
fn check_privacy() -> ExitCode {
    let problems = privacy::check(&root());
    if problems.is_empty() {
        println!("私密路径：被跟踪的文本里没有带真实用户名的路径");
        return ExitCode::SUCCESS;
    }
    for problem in &problems {
        eprintln!("私密路径：{problem}");
    }
    ExitCode::from(1)
}
