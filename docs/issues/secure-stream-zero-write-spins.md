# 下位 stream が 0 byte の書き込みを返し続けると OmniSecureStream の送信が戻らない

**深刻度: 高**（下位 stream が `Ok(0)` を返し続ける場合に限る。コードを読んで確認し、実行による再現は行っていない）

調査日 2026-10-11 / 対象コミット `8ed17cf`

[ISSUES.md](../ISSUES.md) の 1 項目。

## 症状

`OmniSecureStream` の `poll_write` と `poll_flush` は、下位 stream の書き込みが `Ok(0)` を返すと、同じ範囲の書き込みをすぐに再び試みる。
下位 stream が `Ok(0)` を返し続けると、1 回の poll の中で進まないまま繰り返し、呼び出し側の `write` と `flush` は戻らない。
再試行が 1 byte 以上の書き込み、`Pending`、エラーのいずれかを返せば、送信は進むか、呼び出し側へ戻る。

## 該当箇所

[stream.rs:326](../../modules/omnikit/src/service/connection/secure/stream.rs#L326) と [stream.rs:333](../../modules/omnikit/src/service/connection/secure/stream.rs#L333) は `poll_write` の、[stream.rs:372](../../modules/omnikit/src/service/connection/secure/stream.rs#L372) と [stream.rs:379](../../modules/omnikit/src/service/connection/secure/stream.rs#L379) は `poll_flush` の、header と body の送信である。
4 か所とも、下位の `poll_write` が返した byte 数を offset に足して loop の先頭へ戻る。

```rust
let n = match tokio::io::AsyncWrite::poll_write(Pin::new(&mut this.writer), cx, &header.buf[header.offset..]) {
    std::task::Poll::Ready(Ok(n)) => n,
    std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
    std::task::Poll::Pending => return std::task::Poll::Pending,
};
header.offset += n;
```

## 原因

返された byte 数が 0 の場合を区別していない。
offset が進まないため、`SendPayload` の状態のまま同じ範囲の書き込みを再び試みる。

## 影響

`Ok(0)` を返し続ける下位 stream の上では、送信が完了せず、呼び出し側の task が戻らない。
読み取り側は、[stream.rs:136](../../modules/omnikit/src/service/connection/secure/stream.rs#L136) と [stream.rs:168](../../modules/omnikit/src/service/connection/secure/stream.rs#L168) で 0 byte の読み取りを EOF またはエラーとして返しており、同じ形の繰り返しは起きない。

## 対応方針

4 か所で、返された byte 数が 0 の場合に `std::io::ErrorKind::WriteZero` のエラーを返す。
`SendPayload` の状態で `Ok(0)` を返し続ける下位 stream を使う test を足し、`write` と `flush` がエラーで戻ることを守る。
header と body の送信は `poll_write` と `poll_flush` で重複しているため、1 つの関数にまとめてから直すと修正箇所が 2 か所で済む。
