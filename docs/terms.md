# core-rs 用語集

## 1. このドキュメントについて

本書は core-rs の語彙の正本であり、語の定義と識別子の対応を持つ。

### 1.1 文書間の責務分担

| 文書 | 受け持つもの |
| --- | --- |
| 本書 | 語の定義、識別子と隣接語の境界 |
| [DESIGN.md](./DESIGN.md#11-文書間の責務分担) | 文書族の地図と設計の理由 |
| [secure-stream.md](./design/secure-stream.md#8-現状と残作業) | secure stream の contract と実装状況 |

## 2. 用語一覧

| 用語 | スコープ | 識別子 | 外部表現 | 指すもの | 使わない別称 |
| --- | --- | --- | --- | --- | --- |
| Schema | 全体 | `.rpf` | `.rpf` file | package、型、field tag、定数と制約を宣言する生成の入力 | 生成された Rust 型 |
| OmniError | 全体 | `omnius_core_base::error::OmniError` | - | crate 固有のエラー型が実装する共通 contract | 共通の具体的 Error 型 |
| OmniSigner | 全体 | `OmniSigner`、`OmniSigner::public_key()` | 保存済み署名鍵 | 署名を作り、その署名鍵の DER 公開鍵を返す model | peer の証明書 |
| RocketPackStruct | 全体 | `RocketPackStruct` | RocketPack wire | 値の符号化と復号を提供する trait | schema |
| 可変長型の制約 | 全体 | rpf の長さ制約 | `[..=max]`、`[min..=max]` | string・bytes の byte 数、Vec の要素数、Map の wire entry 数が取り得る包含範囲 | message 全体の上限 |
| 制約なしの可変長型 | 全体 | 長さ制約のない rpf 型 | レンジの省略 | schema が長さ上限を定めていない可変長型 | 無制限な入力 |
| 鍵世代 | secure-stream | generation | V2 record header | 送信方向ごとの鍵・IV と sequence の寿命を識別する u64 | node identity の世代 |
| [OmniSecureAuth](./terms/secure-stream.md#t-omni-secure-auth) | secure-stream | `OmniSecureAuth` | `AuthType` | Anonymous または Mutual を接続前に固定する認証設定 | signer の暗黙の有無 |
| OmniSecureStream | secure-stream | `OmniSecureStream` | secure connection | 呼び出し側の byte stream に認証・暗号化・終了処理を重ねる非同期 stream | TCP 接続 |
| OmniSecureStreamOption | secure-stream | `OmniSecureStreamOption` | - | context、handshake の上限と送信側の更新しきい値を設定する入力 | peer が決める上限 |
| record | secure-stream | secure encoder / decoder | V2 record | 1 つの header と認証済み暗号文からなる secure 層の送受信単位 | application message |
| context | secure-stream | `V2ProfileMessage.context` | V2 handshake | 利用側が固定し、認証と鍵導出で通信の用途を区別する byte 列 | peer が選ぶ用途 |
| Close | secure-stream | `V2_RECORD_CLOSE` | V2 record | 送信方向の正常終了を認証する record | transport EOF |
| Data | secure-stream | `V2_RECORD_DATA` | V2 record | 非空の application plaintext を運ぶ record | application frame |
| KeyUpdate | secure-stream | `V2_RECORD_KEY_UPDATE` | V2 record | 次世代への切り替えを旧鍵で通知する record | 再 handshake |

## 3. 隣接語の境界

<a id="t-record-vs-frame"></a>
#### record と application frame

**境界**
record は secure 層の暗号化単位であり、application frame は上位の message 境界である。
1 つの frame は複数 record と鍵世代をまたいでよい。

**取り違えると何が起きるか**
record を message 境界として上位へ返すと、分割と鍵更新だけで application の decode が途中の値を受け取る。

<a id="t-key-update-vs-identity"></a>
#### 鍵更新と identity

**境界**
鍵更新は接続の方向別秘密・鍵・IV の寿命を進める。
identity は保存済みの Ed25519 署名鍵が示す主体であり、record の鍵世代では変更しない。

**取り違えると何が起きるか**
通常の rekey で署名鍵を生成し直すと、利用側が保存した公開鍵と一致しなくなる。
