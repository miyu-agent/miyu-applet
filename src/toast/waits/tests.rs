use super::*;

#[test]
fn the_same_tag_stops_the_one_before() {
    let mut waits = Waits::default();
    let (_, mut first) = waits.start("a:done");
    let (_, mut other) = waits.start("b:done");
    let (_, mut second) = waits.start("a:done");
    assert_eq!(first.try_recv(), Ok(()), "换掉的那一条别等了");
    assert!(other.try_recv().is_err(), "别的 tag 照等");
    assert!(second.try_recv().is_err(), "新的那一条照等");
    assert_eq!(waits.len(), 2);
}

#[test]
fn past_the_limit_the_oldest_stops() {
    let mut waits = Waits::default();
    let mut all: Vec<_> = (0..LIMIT + 3)
        .map(|n| waits.start(&format!("s{n}:done")).1)
        .collect();
    assert_eq!(waits.len(), LIMIT);
    for (n, stopped) in all.iter_mut().enumerate() {
        assert_eq!(stopped.try_recv().is_ok(), n < 3, "第 {n} 条");
    }
}

#[test]
fn done_ones_are_forgotten() {
    let mut waits = Waits::default();
    let (ticket, _stopped) = waits.start("a:done");
    waits.done(ticket);
    assert_eq!(waits.len(), 0);
    // 已经叫停、走了的，再来一条同 tag 的不出错。
    let (_, gone) = waits.start("b:done");
    drop(gone);
    waits.start("b:done");
    assert_eq!(waits.len(), 1);
}
