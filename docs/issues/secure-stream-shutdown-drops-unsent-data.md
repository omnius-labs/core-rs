# OmniSecureStream の shutdown が未送信のデータを送らずに下位 stream を閉じる

**深刻度: 高**（未送信のデータを残したまま shutdown する呼び出し側で起きる。コードを読んで確認し、実行による再現は行っていない）

調査日 2026-10-11 / 対象コミット `8ed17cf`

[ISSUES.md](../ISSUES.md) の 1 項目。

## 症状

`write` したデータが `OmniSecureStream` の中に未送信のまま残っている状態で `shutdown` を呼ぶと、`shutdown` はそのデータを送らずに下位 stream を閉じる。
下位 stream の shutdown が成功した場合、`shutdown` は成功を返し、未送信のデータがあったことをエラーとして報告しない。

## 該当箇所

[stream.rs:395](../../modules/omnikit/src/service/connection/secure/stream.rs#L395) の `poll_shutdown` は、`write_state` を見ずに下位の writer の `poll_shutdown` を呼び、その結果を返す。

```rust
fn poll_shutdown(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
    let this = Pin::into_inner(self);
    trace!("poll_shutdown: {:?}", this.write_state);
    tokio::io::AsyncWrite::poll_shutdown(Pin::new(&mut this.writer), cx)
}
```

## 原因

[stream.rs:304](../../modules/omnikit/src/service/connection/secure/stream.rs#L304) の `poll_write` は、受け取った平文を `WritePlaintext` の状態に溜める。
平文が 64 KiB に達すると暗号化して `SendPayload` の状態へ移るが、その呼び出しでは下位 stream へ送らずに戻る。
`SendPayload` の frame を下位 stream へ送るのは、その後の `poll_write` と `poll_flush` である。
64 KiB に満たない平文を暗号化して送るのは、[stream.rs:349](../../modules/omnikit/src/service/connection/secure/stream.rs#L349) の `poll_flush` だけである。
`poll_shutdown` はどちらの送信も行わないため、`WritePlaintext` の平文と `SendPayload` の frame が `write_state` に残ったまま、下位 stream の書き込み側が閉じる。

## 影響

未送信のデータを残したまま閉じた呼び出し側では、そのデータが相手に届かない。
相手の読み取りの結果は、frame がどこまで届いていたかで分かれる。
frame の header が 1 byte も届いていなければ、相手は [stream.rs:136](../../modules/omnikit/src/service/connection/secure/stream.rs#L136) の分岐で frame の境界の EOF を受け取り、`OmniSecureStream` の読み取りはデータの欠落を示すエラーを返さない。
header または body の途中まで届いていれば、同じ分岐と [stream.rs:168](../../modules/omnikit/src/service/connection/secure/stream.rs#L168) の分岐で `UnexpectedEof` のエラーを受け取る。

[framed_sender.rs:56](../../modules/omnikit/src/service/connection/codec/framed_sender.rs#L56) の `FramedSender::send` は、message の書き込みに続けて flush を行う。
`send` が成功を返した message は、`OmniSecureStream` の中に残らない。

## 対応方針

`poll_shutdown` で、下位の `poll_shutdown` を呼ぶ前に `poll_flush` と同じ送信を完了させる。
flush を呼ばずに `shutdown` した後、相手が書き込まれた全データを受信できることを、shutdown が成功する下位 stream を使う test で守る。
