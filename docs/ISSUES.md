# core-rs の既知の不具合

本書は、core-rs のコードで確認した明確な不具合だけを扱う。
依存する library や OS の振る舞いそのものは扱わない。
workspace 横断の設計判断と将来構想は [DESIGN.md](./DESIGN.md#11-設計判断) が扱う。
RocketPack compiler 固有の論点は [RocketPack compiler の設計](./design/rocketpack-compiler.md#10-設計判断) が扱う。

項目を外部 Issue に起票した場合は一覧の Issue 列へ番号を追記し、修正された場合は一覧の行と `docs/issues/` の子文書を削除する。
行番号は調査時点のスナップショットであるため、修正時に再確認する。

| 項目 | 深刻度 | 調査日 | Issue |
| --- | --- | --- | --- |
| [下位 stream が 0 byte の書き込みを返し続けると OmniSecureStream の送信が戻らない](./issues/secure-stream-zero-write-spins.md) | 高 | 2026-10-11 | - |
| [OmniSecureStream の shutdown が未送信のデータを送らずに下位 stream を閉じる](./issues/secure-stream-shutdown-drops-unsent-data.md) | 高 | 2026-10-11 | - |

2 項目はどちらも `OmniSecureStream` の書き込み側にあり、header と body を下位 stream へ送る同じ処理に関わる。
送信の処理を 1 つにまとめる変更を先に行うと、両方を同じ箇所で直せる。

## ここに含めていないもの

| 論点 | 設計上の扱い |
| --- | --- |
| Remote dependency と lockfile | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
| 複数の Rust module root | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
