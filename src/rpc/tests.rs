use super::*;

use tokio::io::{DuplexStream, ReadHalf, WriteHalf, duplex};

type Pipe = Rpc<ReadHalf<DuplexStream>, WriteHalf<DuplexStream>>;

/// 托盘这头一条 `Rpc`，另一头当核心：读托盘写的、写回去。
fn pair() -> (
    Pipe,
    BufReader<ReadHalf<DuplexStream>>,
    WriteHalf<DuplexStream>,
) {
    let (ours, theirs) = duplex(64 * 1024);
    let (read, write) = tokio::io::split(ours);
    let (core_read, core_write) = tokio::io::split(theirs);
    (Rpc::new(read, write), BufReader::new(core_read), core_write)
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .unwrap()
}

async fn asked(core: &mut BufReader<ReadHalf<DuplexStream>>) -> Value {
    let mut line = String::new();
    core.read_line(&mut line).await.unwrap();
    serde_json::from_str(&line).unwrap()
}

async fn say(core: &mut WriteHalf<DuplexStream>, message: &Value) {
    core.write_all(format!("{message}\n").as_bytes())
        .await
        .unwrap();
}

#[test]
fn a_reply_is_matched_by_its_id_and_pushes_in_between_are_kept() {
    runtime().block_on(async {
        let (mut rpc, mut core_read, mut core_write) = pair();
        let core = async {
            let request = asked(&mut core_read).await;
            assert_eq!(request["method"], "config.get");
            assert!(request["id"].as_str().unwrap().starts_with("app-"));
            say(
                &mut core_write,
                &json!({"jsonrpc": "2.0", "method": "package.changed", "params": {"package": "a"}}),
            )
            .await;
            say(
                &mut core_write,
                &json!({"jsonrpc": "2.0", "id": "other", "result": {}}),
            )
            .await;
            say(
                &mut core_write,
                &json!({"jsonrpc": "2.0", "id": request["id"], "result": {"items": 1}}),
            )
            .await;
            say(
                &mut core_write,
                &json!({"jsonrpc": "2.0", "method": "config.changed", "params": {}}),
            )
            .await;
        };
        let ours = async {
            let mut slow = |_: bool| panic!("不该慢");
            let got = rpc.call("config.get", json!({}), &mut slow).await;
            assert_eq!(got, Ok(json!({"items": 1})));
            assert_eq!(rpc.push().await.unwrap()["method"], "package.changed");
            assert_eq!(rpc.push().await.unwrap()["method"], "config.changed");
        };
        tokio::join!(core, ours);
    });
}

#[test]
fn ids_differ_between_calls_and_between_connections() {
    runtime().block_on(async {
        let (mut first, mut core_read, mut core_write) = pair();
        let (second, _, _) = pair();
        assert_ne!(first.prefix, second.prefix);
        let core = async {
            for _ in 0..2 {
                let request = asked(&mut core_read).await;
                say(
                    &mut core_write,
                    &json!({"jsonrpc": "2.0", "id": request["id"], "result": request["id"]}),
                )
                .await;
            }
        };
        let ours = async {
            let mut slow = |_: bool| {};
            let one = first.call("a", json!({}), &mut slow).await.unwrap();
            let two = first.call("b", json!({}), &mut slow).await.unwrap();
            assert_ne!(one, two);
        };
        tokio::join!(core, ours);
    });
}

#[test]
fn a_refusal_carries_the_reason_and_the_sentence() {
    let reply = json!({"id": "x", "error": {"code": -32010, "message": "没有这一项。", "data": {"reason": "unknown_config_key"}}});
    assert_eq!(
        answer(&reply),
        Err(Failure::Refused {
            reason: "unknown_config_key".into(),
            message: "没有这一项。".into()
        })
    );
    assert_eq!(
        answer(&json!({"id": "x", "error": {}})),
        Err(Failure::Refused {
            reason: String::new(),
            message: "refused".into()
        })
    );
}

#[test]
fn a_late_reply_says_slow_then_answers() {
    runtime().block_on(async {
        let (mut rpc, mut core_read, mut core_write) = pair();
        let core = async {
            let request = asked(&mut core_read).await;
            // 回一半：等过了 SLOW 再写完，读了一半的不能丢。
            let reply =
                json!({"jsonrpc": "2.0", "id": request["id"], "result": {"ok": true}}).to_string();
            let (head, tail) = reply.split_at(10);
            core_write.write_all(head.as_bytes()).await.unwrap();
            tokio::time::sleep(SLOW * 2).await;
            core_write
                .write_all(format!("{tail}\n").as_bytes())
                .await
                .unwrap();
        };
        let ours = async {
            let mut said = Vec::new();
            let got = rpc
                .call("hello", json!({}), &mut |slow| said.push(slow))
                .await;
            assert_eq!(got, Ok(json!({"ok": true})));
            assert_eq!(said, [true, false]);
        };
        tokio::join!(core, ours);
    });
}

#[test]
fn a_closed_connection_ends_the_call() {
    runtime().block_on(async {
        let (mut rpc, core_read, core_write) = pair();
        drop((core_read, core_write));
        let got = rpc.call("hello", json!({}), &mut |_| {}).await;
        assert!(matches!(got, Err(Failure::Closed(_))), "{got:?}");
        assert!(matches!(rpc.push().await, Err(Failure::Closed(_))));
    });
}

#[test]
fn hello_shows_the_local_token_and_says_it_is_the_tray() {
    let hello = hello("t0k", "zh_CN.UTF-8");
    assert_eq!(hello["token"], "t0k");
    assert_eq!(hello["protocol"], json!([1, 1]));
    assert_eq!(hello["head"]["kind"], "miyu-applet");
    assert_eq!(hello["locale"], "zh_CN.UTF-8");
}

#[test]
fn lines_are_replies_pushes_or_nothing() {
    assert!(matches!(
        classify(r#"{"id":"a","result":{}}"#),
        Line::Reply(_)
    ));
    assert!(matches!(
        classify(r#"{"method":"resync","params":{}}"#),
        Line::Push(_)
    ));
    assert_eq!(classify(r#"{"id":null}"#), Line::Other);
    assert_eq!(classify("not json"), Line::Other);
}

/// 审计 #100：一直不换行的一行不跟着攒内存，读过上限就当断了（原来一直读、一直攒）。
#[test]
fn a_line_that_never_ends_is_cut_off() {
    runtime().block_on(async {
        let (mut rpc, _core_read, mut core_write) = pair();
        let core = async {
            let chunk = vec![b'x'; 64 * 1024];
            for _ in 0..(17 * 16) {
                core_write.write_all(&chunk).await.unwrap();
            }
            // 不换行、不断开：还连着。
            std::future::pending::<()>().await;
        };
        let ours = async {
            let got = tokio::time::timeout(Duration::from_secs(60), rpc.push()).await;
            assert!(
                matches!(got, Ok(Err(Failure::Closed(_)))),
                "读过上限要当断了：{got:?}"
            );
        };
        tokio::select! {
            () = core => {}
            () = ours => {}
        }
    });
}
