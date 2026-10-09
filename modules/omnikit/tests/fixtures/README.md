# OmniSecureStream V2 の固定値

[詳細仕様](../../../../docs/design/secure-stream.md#3-handshake) に対して、production の Rust code を使わずに計算した期待値を置く。
固定値は暗号計算と wire の照合に使い、I/O、cancellation と攻撃試験の代わりにはしない。

## 内容

`secure_v2_vectors.json` は Mutual と Anonymous の 2 case を持つ。
`inputs` の private key と seed は公開された試験専用の値であり、実際の identity と通信には使わない。
`canonical` は意味的な profile と auth、`handshake_wire` は magic と長さ header を含む各送信値、`crypto` は中間 hash と方向別秘密・鍵・IV を持つ。
`records` は各方向の Data、KeyUpdate、更新後の Data と Close の値を持つ。
各方向で generation 0 の sequence 0・1 を送り、更新後の generation 1 で sequence 0・1 を送る。

`negative_records` の `before` は、違反を検査する前に受理する `records` の名前を表す。
`error` は仕様上の拒否理由であり、公開 Rust エラー型の名前を定めるものではない。
更新通知の違反例は正常な tag を持ち、復号だけでなく更新状態の検査を必要とする。
`legacy_signatures_hex` は接頭辞のない T0 の 32 byte への署名であり、V2 の署名検証では拒否する。

## 再現

Python 3.13 と `cryptography==50.0.2` を使う。
Python の環境は `AGENT_TEMP_DIR` 配下など workspace 外へ作り、依存は `requirements-secure-v2.txt` から導入する。
次のコマンドは core-rs の root で実行し、`python` はその環境の interpreter とする。

```sh
python modules/omnikit/tests/fixtures/secure_v2_vectors.py --check modules/omnikit/tests/fixtures/secure_v2_vectors.json
```

生成器は RFC 5869 の recurrence で求めた HKDF-Expand を cryptography の API と照合し、双方の X25519、署名・旧署名の拒否、AES-GCM の復号と不正 tag の拒否、GHASH の入力上限を検査する。
Rust の生成 codec と別の暗号計算による照合は、独立 verifier が行う。
`RUSTC_WRAPPER= cargo test -p omnius-core-omnikit --test secure_v2_vectors` は、生成 codec と固定 wire の一致、署名と役割・transcript の束縛、旧署名の拒否を継続検査する。
`RUSTC_WRAPPER= cargo test -p omnius-core-omnikit --all-features` は production の handshake・record・rekey、constructor の攻撃拒否、公開時点、各段階の中断、I/O と使用量・終了の境界も検査する。
固定値の再生成は同じコマンドの `--check` を `--write` に置き換える。
