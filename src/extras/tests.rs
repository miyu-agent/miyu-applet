use super::*;

use serde_json::json;

fn listed() -> Value {
    json!({"packages": [
        {"package": "im", "kind": "process", "enabled": true, "status": "running", "page": true, "tray": {"name": "打开通讯平台接入界面"}},
        {"package": "hello", "kind": "process", "enabled": false, "status": "off", "page": true, "tray": {"name": "打开打招呼"}},
        {"package": "aaa", "kind": "builtin", "page": true, "status": "ready", "tray": {"name": "  看一眼  "}},
        {"package": "no-tray", "kind": "process", "enabled": true, "status": "running", "page": true},
        {"package": "broken", "layer": "home", "code": "bad_tray", "problem": "…", "line": 3},
    ]})
}

fn names(packages: &Packages) -> Vec<(String, String)> {
    packages
        .entries()
        .into_iter()
        .map(|entry| (entry.package, entry.name))
        .collect()
}

#[test]
fn only_switched_on_packages_with_a_tray_line_are_listed_by_id() {
    let mut packages = Packages::default();
    packages.replace(&listed());
    assert_eq!(
        names(&packages),
        [
            ("aaa".to_string(), "看一眼".to_string()),
            ("im".to_string(), "打开通讯平台接入界面".to_string()),
        ]
    );
}

#[test]
fn a_change_replaces_or_removes_one_package() {
    let mut packages = Packages::default();
    packages.replace(&listed());
    // 只是变了的不算装卸。
    assert!(!packages.change(&json!({"package": "hello", "entry": {"package": "hello", "enabled": true, "status": "starting", "tray": {"name": "打开打招呼"}}})));
    assert!(packages.change(&json!({"package": "im", "entry": null})));
    assert!(!packages.change(&json!({"package": "aaa", "entry": {"package": "aaa", "status": "program_missing", "tray": {"name": "看一眼"}}})));
    assert!(packages.change(&json!({"package": "new", "entry": {"package": "new"}})));
    assert!(!packages.change(&json!({"package": "gone", "entry": null})));
    assert_eq!(
        names(&packages),
        [("hello".to_string(), "打开打招呼".to_string())]
    );
    // 认不出的推送不动列表。
    assert!(!packages.change(&json!({"entry": null})));
    assert_eq!(packages.entries().len(), 1);
}

#[test]
fn an_empty_or_odd_reply_lists_nothing() {
    let mut packages = Packages::default();
    packages.replace(&json!({}));
    assert!(packages.entries().is_empty());
    packages.replace(&json!({"packages": [{"package": "x", "tray": {"name": ""}}, {"tray": {"name": "没编号"}}]}));
    assert!(packages.entries().is_empty());
}

#[test]
fn loopback_is_off_and_anything_else_is_on() {
    for off in [
        "127.0.0.1",
        "127.1.2.3",
        "::1",
        "[::1]",
        "localhost",
        " ",
        "",
    ] {
        assert!(!lan_on(&json!(off)), "{off}");
    }
    assert!(!lan_on(&Value::Null));
    for on in ["0.0.0.0", "::", "192.168.1.5", " 10.0.0.2 "] {
        assert!(lan_on(&json!(on)), "{on}");
    }
}

#[test]
fn the_config_reply_says_on_off_or_unknown() {
    let on = json!({"items": {"web.listen": {"value": "0.0.0.0", "origin": {"layer": "system"}}}});
    let off =
        json!({"items": {"web.listen": {"value": "127.0.0.1", "origin": {"layer": "default"}}}});
    assert_eq!(lan_of(&on), Some(true));
    assert_eq!(lan_of(&off), Some(false));
    assert_eq!(lan_of(&json!({"items": {}})), None);
}

#[test]
fn only_changes_to_the_listen_address_count() {
    assert!(touches_listen(&json!({"web.listen": {}})));
    assert!(!touches_listen(&json!({"ui.language": {}})));
    assert!(!touches_listen(&Value::Null));
}

#[test]
fn a_version_comes_from_the_listed_package_and_follows_changes() {
    let mut packages = Packages::default();
    packages.replace(&json!({"packages": [
        {"package": "webui", "version": "0.1.0", "layer": "home"},
        {"package": "blank", "version": "  "},
        {"package": "nameless"},
    ]}));
    assert_eq!(packages.version("webui").as_deref(), Some("0.1.0"));
    assert_eq!(packages.version("blank"), None);
    assert_eq!(packages.version("nameless"), None);
    assert_eq!(packages.version("missing"), None);
    // 换成另一层的那一份（核心照层挑好推来的）：照新的
    packages.change(&json!({"package": "webui", "entry": {"package": "webui", "version": "0.2.0", "layer": "system"}}));
    assert_eq!(packages.version("webui").as_deref(), Some("0.2.0"));
    packages.change(&json!({"package": "webui", "entry": null}));
    assert_eq!(packages.version("webui"), None);
}

#[test]
fn a_package_covered_by_a_higher_layer_is_not_the_one_read() {
    let mut packages = Packages::default();
    // 核心 SD-1补：认的那份在前，被盖住的接在后面、带 shadowed 对象；F-9 的 covered: true 不算被盖住
    packages.replace(&json!({"packages": [
        {"package": "webui", "layer": "home", "version": "0.2.0"},
        {"package": "webui", "layer": "distro", "shadowed": {"by": "home", "version": "0.3.0", "over": "0.2.0", "newer": true}},
        {"package": "kit", "layer": "home", "version": "1.0", "covered": true},
    ]}));
    assert_eq!(packages.version("webui").as_deref(), Some("0.2.0"));
    assert_eq!(packages.version("kit").as_deref(), Some("1.0"));
    // 推来被盖住那份的变动：认的那份不动
    let changed = packages.change(&json!({"package": "webui", "entry": {"package": "webui", "layer": "distro", "shadowed": {"by": "home"}}}));
    assert!(!changed);
    assert_eq!(packages.version("webui").as_deref(), Some("0.2.0"));
}
