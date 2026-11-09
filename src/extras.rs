//! 菜单上照核心来的两组（图纸 `app.md` 第 4 条）：软件包挂的入口（核心 TR-2）、局域网（网页的 `web.listen`）。只有读和算，不碰界面、
//! 不碰连接，单测钉住。
//!
//! - 软件包：`subscribe {stream: "packages"}` 的回应是整份列表，之后推 `package.changed {package, entry}`（没有了的 `entry` 是 `null`）。
//!   一项带 `tray: {name}`（有后台页、清单写了 `[tray]`，名字照连接的语言挑好的）、开着的才列，一个包一行，照编号排（同 `package.list`）。
//! - 局域网：`config.get {keys: ["web.listen"]}`；照网页设置页的认法（网页 `account.js` 的 `loopbackOnly`）：回环（`127.*`、`::1`、
//!   `localhost`、空的）是关，别的（`0.0.0.0`、这台电脑的某一个地址）是开。

use std::collections::BTreeMap;

use serde_json::Value;

/// 跟着的配置项：网页软件听哪个地址。
pub const LISTEN: &str = "web.listen";
/// 打开局域网写的值（同网页设置页）。关掉是恢复默认（`unset`，出厂的 `127.0.0.1`）。
pub const ANY: &str = "0.0.0.0";

/// 托盘菜单里一个软件包的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// 包的编号：点了 `miyu pkg page <编号>`。
    pub package: String,
    /// 那一行的字。
    pub name: String,
}

/// 照 `packages` 流记着的软件包列表（只留算入口要的那一项项）。
#[derive(Debug, Default)]
pub struct Packages {
    items: BTreeMap<String, Value>,
}

impl Packages {
    /// 订阅的回应：整份换掉。同一个编号几层都有的（核心 SD-1），被上面一层盖住的那份也列着、带 `shadowed` 对象，只认没被盖住的
    /// 那份（[`shadowed`]）。
    pub fn replace(&mut self, reply: &Value) {
        self.items = reply["packages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| !shadowed(item))
            .filter_map(|item| Some((item["package"].as_str()?.to_string(), item.clone())))
            .collect();
    }

    /// 推来的 `package.changed` 的参数：换掉、删掉那一项。交回是不是装上、卸掉了一个包（不只是状态变了）：装卸包不推
    /// `config.changed`，包带的配置项（例如网页软件的 `web.listen`）有没有跟着它变。
    pub fn change(&mut self, params: &Value) -> bool {
        let Some(package) = params["package"].as_str() else {
            return false;
        };
        match &params["entry"] {
            // 被盖住的那份变了：托盘认的那份不动
            entry if shadowed(entry) => false,
            Value::Null => self.items.remove(package).is_some(),
            entry => self
                .items
                .insert(package.to_string(), entry.clone())
                .is_none(),
        }
    }

    /// 装着的一个包的版本（`package.list` 那一项的 `version`）：核心照层挑好的那一份（出厂、系统加装、装上的，后面的同编号盖过
    /// 前面的；核心设计 60）。没有这个包、没写版本的是 `None`。
    pub fn version(&self, package: &str) -> Option<String> {
        let version = self.items.get(package)?["version"].as_str()?.trim();
        (!version.is_empty()).then(|| version.to_string())
    }

    /// 托盘菜单该列的那几行，照编号排。
    pub fn entries(&self) -> Vec<Entry> {
        self.items
            .iter()
            .filter(|(_, item)| switched_on(item))
            .filter_map(|(package, item)| {
                let name = item["tray"]["name"].as_str()?.trim();
                (!name.is_empty()).then(|| Entry {
                    package: package.clone(),
                    name: name.to_string(),
                })
            })
            .collect()
    }
}

/// 被上面一层盖住的那份（核心 SD-1补 的 `shadowed` 对象）。`covered: true` 是另一回事（核心 F-9 补：包里的 MCP 服务全被自制的同名
/// 服务替代了，认的那份也可能有），不算。
fn shadowed(item: &Value) -> bool {
    item["shadowed"].is_object()
}

/// 开着没有：开关关着的（`enabled: false`）、卸掉了的出厂包、`status` 是关着、程序不在、缺依赖的不算；没有开关的（内置包）算开着。
fn switched_on(item: &Value) -> bool {
    item["enabled"] != Value::Bool(false)
        && item["removed"] != Value::Bool(true)
        && !matches!(
            item["status"].as_str(),
            Some("off" | "program_missing" | "depends_missing")
        )
}

/// `web.listen` 的值是不是听局域网。
pub fn lan_on(listen: &Value) -> bool {
    let value = listen
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    !(value.is_empty()
        || value == "localhost"
        || value == "::1"
        || value == "[::1]"
        || value.starts_with("127."))
}

/// `config.get` 的回应里局域网开没开；没有这一项的（网页软件没装）交 `None`。
pub fn lan_of(reply: &Value) -> Option<bool> {
    reply["items"]
        .get(LISTEN)
        .map(|item| lan_on(&item["value"]))
}

/// 推来的 `config.changed` 的 `keys` 里有没有 `web.listen`。
pub fn touches_listen(keys: &Value) -> bool {
    keys.get(LISTEN).is_some()
}

#[cfg(test)]
mod tests;
