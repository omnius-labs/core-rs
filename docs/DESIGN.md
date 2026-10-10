# core-rs 設計書

## 1. このドキュメントについて

本書は、core-rs workspace を構成する各 crate（`omnius-core-*`）と entrypoint の責務境界、不変条件、設計判断、実装状況を扱う。
対象読者は、いずれかの crate や entrypoint を変更する実装者と reviewer である。

### 1.1 文書間の責務分担

| 文書または正本 | 受け持つもの |
| --- | --- |
| 本書 | workspace 全体の構成、crate 間の責務境界、不変条件、設計判断、実装状況 |
| [ISSUES.md](./ISSUES.md) | コードで確認した明確な不具合と起票までの作業用一覧 |
| [RocketPack compiler の設計](./design/rocketpack-compiler.md) | compiler 固有の責務、不変条件、設計判断、実装状況 |
| [entrypoints/rocketpack-compiler](../entrypoints/rocketpack-compiler) | `.rpf` の構文、意味検査、Rust code generation の実装 |
| [modules/rocketpack](../modules/rocketpack) | wire codec と生成コードが利用する runtime API の実装 |
| [modules/omnikit/rpfs](../modules/omnikit/rpfs) | omnikit プロトコルが使う `.rpf` schema の正本 |
| [entrypoints/rocketpack-compiled-example/showcase/rpfs](../entrypoints/rocketpack-compiled-example/showcase/rpfs) | 生成例が利用する `.rpf` schema の正本 |
| [README.md](../README.md) | プロジェクト概要と外部ドキュメントへのリンク |

### 1.2 本書の時制について

本文は合意済みの完成形を現在形で記述する。
実装状況と残作業は §12 に集約する。
いま何が動くのかを知りたい場合は §12 を先に読む。

## 2. core-rs とは

core-rs は、Omnius Labs の各プロダクトが共有する Rust 製 crate 群の workspace である。
独自の wire codec（RocketPack）とその上に構築したセキュアな接続、リモート呼び出し（omnikit）、stream 多重化（yamux）というネットワーク関連の crate 群と、クラウドサービス連携や DB migration などの運用基盤（cloud、migration、testkit）を、それぞれ独立した crate として提供する。

### 2.1 スコープ外

- 各 crate はライブラリであり、単体で動作するサーバーやデーモンの実行 binary を持たない。
  `rocketpack-compiler` だけは schema からコードを生成する CLI として例外的に binary を持つが、これは compile 時ツールであり、実行時に何かを配信する server ではない。
  持たないことで、本書はプロセス起動やデプロイ方式を扱わない。
- omnikit プロトコルを実際に話す P2P node や server の実装は持たない。
  持たないことで、node 発見やネットワークトポロジーの設計は本書の対象外である。
- cloud と migration は、workspace 内の他 crate から利用されていない。
  cloud の `#[ignore]` 付き統合テストが `opxs-dev`、`opxs-batch-email-send-sqs` など外部プロダクト固有のリソース名を含むことから、これらは core-rs の外にある downstream アプリケーションから個別に利用される想定に読める（コード上に明示の記述はない）。
  持たないことで、本書はそれらアプリケーション固有のドメインロジックや認証情報の管理方式を扱わない。
- C# と Swift 向けの RocketPack generator は持たない。
  `rocketpack-compiler` は Rust 向け生成のみを実装する。
  持たないことで、本書は他言語向けの生成 API とその検証方式を定めない。

## 3. 全体構成

### 3.1 コンポーネントとディレクトリ対応表

| パス | package | 役割 |
| --- | --- | --- |
| [modules/base](../modules/base) | `omnius-core-base` | cloud、image、migration、omnikit、testkit、yamux が依存する基盤（`Clock`、`OmniError` contract、file lock、sleeper、tsid など） |
| [modules/rocketpack](../modules/rocketpack) | `omnius-core-rocketpack` | wire codec と `RocketPackStruct` の runtime contract |
| [modules/omnikit](../modules/omnikit) | `omnius-core-omnikit` | RocketPack 上に構築したセキュアな接続とリモート呼び出し（omnikit プロトコル） |
| [modules/yamux](../modules/yamux) | `omnius-core-yamux` | yamux stream 多重化の Tokio 向け wrapper |
| [modules/cloud](../modules/cloud) | `omnius-core-cloud` | AWS（S3、Secrets Manager、SES、SQS）と GCP（Secret Manager）の薄い wrapper |
| [modules/migration](../modules/migration) | `omnius-core-migration` | PostgreSQL と SQLite 向けの自前 migration runner |
| [modules/testkit](../modules/testkit) | `omnius-core-testkit` | 統合テスト用の Docker container 起動 helper |
| [modules/image](../modules/image) | `omnius-core-image` | EXIF metadata 関連の依存関係のみを宣言した crate（詳細は §12） |
| [entrypoints/rocketpack-compiler](../entrypoints/rocketpack-compiler) | `omnius-core-rocketpack-compiler` | `.rpf` の parse、意味検査、Rust code generation を行う CLI |
| [entrypoints/rocketpack-compiled-example](../entrypoints/rocketpack-compiled-example) | 該当なし（workspace から除外） | 生成コードの実例と round trip 検証用の sample project。`showcase`、`provider`、`consumer` の 3 project を同じ構造で並べ、`provider` と `consumer` は manifest 間の path dependency を検証する |

### 3.2 処理の流れ

```mermaid
flowchart LR
    base[base]

    subgraph protocol["プロトコル層"]
        rocketpack[rocketpack]
        omnikit[omnikit]
        yamux[yamux]
    end

    subgraph infra["運用基盤層"]
        cloud[cloud]
        migration[migration]
        testkit[testkit]
        image[image]
    end

    rpf["modules/omnikit/rpfs/*.rpf"] --> compiler["rocketpack-compiler"] --> gen["modules/omnikit/src/generated"]

    base --> omnikit
    base --> yamux
    base --> cloud
    base --> migration
    base --> testkit
    base --> image
    rocketpack --> omnikit
    gen --> omnikit
    migration -. dev dependency .-> testkit
```

rocketpack と rocketpack-compiler は base に依存せず、OmniError の contract にも従わない（§4.1）。
yamux は base にのみ依存する独立した crate であり、omnikit の remoting 層とは結線されていない。
remoting 層をどう多重化するかは §11.2 で保留している。

## 4. 中心概念

### 4.1 OmniError

**OmniError** は、crate が自分のエラー型に実装する共通のエラー contract である。
コード上は `omnius_core_base::error::OmniError` という、associated type `ErrorKind` を持つ trait であり、`new`、`from_error`、`with_message`、`kind`、`message`、`backtrace` という API と、既定の `Debug` 形式の `fmt` を持つ。
`cloud`、`migration`、`omnikit`、`testkit`、`yamux` はそれぞれ独立した `crate::error::Error` と `crate::error::ErrorKind` の組を用意してこの trait を実装し、`crate::result::Result<T>` をその `Error` に対する alias として `crate::prelude` から re-export する。
`rocketpack` と `rocketpack-compiler` はこの規約に従わず、`thiserror::Error` で定義した専用の error 型（`RocketPackEncoderError` など）を直接使う。
OmniError は trait であり具体的な型ではないため、workspace 全体で共有される単一の Error 型は存在しない。

### 4.2 RocketPackStruct

**RocketPackStruct** は、値を wire へ pack し、wire から unpack する構造体が実装する contract である。
コード上は `omnius_core_rocketpack::RocketPackStruct` という trait であり、`validate`、`pack`、`unpack`、`import`、`export` を持つ。
`rocketpack-compiler` が生成する Rust 型はすべてこの trait を実装する。
omnikit の remoting 層（`OmniRemotingStream::send` と `recv`）は、この trait を型境界に持つ generic 関数として、生成された任意のメッセージ型を送受信する。
Schema はこの trait の実装を生成させる入力であり、RocketPackStruct は生成された出力側が満たす契約である。

### 4.3 Schema

**Schema** は `.rpf` に記述した package、型、field tag、定数、制約の集合である。
`.rpf` が正本であり、生成された Rust code は schema から再生成できる派生成果物である。
[modules/omnikit/rpfs](../modules/omnikit/rpfs) と [entrypoints/rocketpack-compiled-example/showcase/rpfs](../entrypoints/rocketpack-compiled-example/showcase/rpfs) は、それぞれ omnikit プロトコルと生成例が使う schema の正本である。

### 4.4 可変長型の制約

**可変長型の制約** は、`string`、`bytes`、`Vec`、`Map` の各値が取り得る長さの包含範囲である。
`string` と `bytes` は byte 数、`Vec` は要素数、`Map` は wire 上の entry 数を長さとする。
`Option` は値の有無だけを表し、固定長 array は外側の長さが型で決まるため、それぞれ内包する可変長型だけが制約を持つ。
制約は各出現箇所で任意である。
制約を書かない可変長型を **制約なしの可変長型** と呼び、schema はその値の長さを制限しない。
制約なしの可変長型に残る唯一の上限は、decoder が入力の残り byte 数に対して行う検査（§5.3）である。

## 5. RocketPack

RocketPack は、`.rpf` schema から Rust のデータ型と wire codec を生成する仕組みである。
schema は wire 上の field tag と型を定め、runtime は CBOR 互換の長さ表現を読み書きする。
message 全体の byte 数、総要素数、nest 深度は codec または transport の別制約とし、field ごとの長さ制約へは集約しない（§11.1 の[長さ制約を各値へ限定する](#d-per-value-scope)を参照）。
core-rs 以外にある `.rpf` の移行は各 repository が所有し、core-rs は repository 間の同期機構を持たない。

### 5.1 コンポーネントとディレクトリ対応表

| パス | package | 役割 |
| --- | --- | --- |
| [modules/rocketpack](../modules/rocketpack) | `omnius-core-rocketpack` | encoder、decoder、`RocketPackStruct` の runtime contract |
| [entrypoints/rocketpack-compiler](../entrypoints/rocketpack-compiler) | `omnius-core-rocketpack-compiler` | `.rpf` の parse、意味検査、Rust code generation |
| [modules/omnikit/rpfs](../modules/omnikit/rpfs) | 該当なし | omnikit プロトコルが使う schema の正本 |
| [modules/omnikit/rocketpack.yaml](../modules/omnikit/rocketpack.yaml) | 該当なし | omnikit の schema module manifest。`rpfs` 配下の全 `.rpf` を [modules/omnikit/src/generated](../modules/omnikit/src/generated) へ生成する |
| [gen-rocketpack.sh](../gen-rocketpack.sh) | 該当なし | repository root から `rocketpack-compiler` を実行して omnikit の生成コードを更新する script |
| [entrypoints/rocketpack-compiled-example/showcase/rpfs](../entrypoints/rocketpack-compiled-example/showcase/rpfs) | 該当なし | 生成例が利用する schema の正本 |
| [entrypoints/rocketpack-compiled-example/showcase/rust/gen](../entrypoints/rocketpack-compiled-example/showcase/rust/gen) | `rocketpack-showcase` | compiler が出力する Rust code であり、手で編集しない |

```mermaid
flowchart LR
    RPF[.rpf schema] --> Parser[parser and semantic validation]
    Parser --> Generator[Rust generator]
    Generator --> Generated[generated Rust types and codec]
    Generated --> Runtime[RocketPack runtime]
    Runtime --> Wire[wire bytes]
```

schema の制約は compiler が一度解決し、生成コードと runtime の境界検査へ反映する。
`rocketpack-compiler` の generator dispatch は、plugin id が `rocketpack-rust` の場合だけ実際に生成する。
`rocketpack-csharp` と `rocketpack-swift` は認識はするが、生成せずに log を出して skip する。

### 5.2 Schema compile 時

compiler は field tag と enum variant tag の重複、type alias 内の制約、default literal を生成前に検査する。
tag の重複は struct の field、enum の variant、record variant 内の field のそれぞれで検査する。
制約を書いた出現箇所については、加えて有限な上限、境界値の解決、範囲の順序を検査する。
どの schema にも不正があれば、生成物を書き出す前に失敗する。

### 5.3 Codec 実行時

生成される field は `String`、`Vec`、`BTreeMap` などの標準 Rust 型を保つ。
encode は length prefix を書く前に検査し、decode は collection の確保、反復、payload の所有化より前に宣言長を検査する。
制約違反は schema path と実際の長さを持つ専用 error として返す。
制約の有無に関わらず、decoder は array と map の宣言長が入力の残り byte 数に収まることを検査する。
この検査は schema 由来の制約ではなく wire の整合性に属するため、schema path を持たない `UnexpectedEof` として返す。

## 6. omnikit

omnikit は、RocketPack で定義した message（[modules/omnikit/rpfs](../modules/omnikit/rpfs)）の上に、鍵交換と認証付き暗号化を行うセキュアな接続と、関数単位のリモート呼び出しを構築する crate である。

### 6.1 層構造

呼び出し側が用意する raw な `T: AsyncRead + AsyncWrite`（TCP や `tokio::io::duplex` など）の上に、次の層を積む。

```
layer 4: remoting (service/remoting)
         HelloMessage handshake と RocketPackStruct message の送受信
----------------------------------------------------------------------
layer 3: secure connection (service/connection/secure)
         X25519 鍵交換 + HKDF + AES-256-GCM
----------------------------------------------------------------------
layer 2: framed codec (service/connection/codec)
         length delimited framing
----------------------------------------------------------------------
layer 1: raw AsyncRead + AsyncWrite（呼び出し側が用意）
```

各層は下の層に対する generic 関数として実装されているため、型のうえでは自由に組み合わせられる。

### 6.2 secure connection

handshake は、双方の接続ごとの値を署名と鍵導出に含め、鍵確認で署名者の公開鍵も照合する。
[auth.rs](../modules/omnikit/src/service/connection/secure/auth.rs) の `Authenticator::auth` がこの処理を担当し、成功した鍵だけを暗号化した frame の層へ渡す。
V1 は本節の手順だけを持ち、旧形式の受理と fallback は行わない。

#### API と接続の向き

`OmniSecureStream::new` は、`stream`、`stream_type`、`max_frame_length`、`signer`、`require_peer_signature: bool`、`max_clock_skew: std::time::Duration`、`clock`、`rng` をこの順で受け取る。
発信側は `OmniSecureStreamType::Connected`、受理側は `Accepted` を指定する。
role はそれぞれ 1 byte の `0x01` と `0x02` で表し、`ProfileMessage` にも含める。
`ProfileMessage` の `role` の schema 型は `u8` とし、decode 後に値を検査する。
相手が自分と同じ role または未知の role を送る接続は拒否する。

`signer` があれば自分の `auth_type` は `Sign`、なければ `None` とする。
`require_peer_signature` が `true` の側は、相手の profile が `None` なら拒否する。
`false` の場合も、相手が `Sign` を申告すれば cert の受信と検証を必須とする。
自分の署名の有無と、相手へ署名を要求するかどうかは独立であり、`AuthType::None` は匿名の鍵交換に使える。

成功後の `peer_cert(&self) -> Option<&OmniCert>` は、handshake で検証し、鍵確認でも公開鍵を照合した相手の cert を返す。
相手が `None` の場合は `None` を返す。
`sign_id()` はその cert から作る表示用の文字列を返す。
期待する相手かどうかの判定は呼び出し側が返された cert の `public_key` と既知の公開鍵を照合して行い、署名の対象に含まれない cert の `name` は認証済みの識別情報として扱わない。

#### Message の順序

```mermaid
sequenceDiagram
    participant C as 発信側 Connected
    participant A as 受理側 Accepted
    C->>A: ProfileMessage(role, session_id, auth_type, algorithm flags)
    A->>C: ProfileMessage(role, session_id, auth_type, algorithm flags)
    C->>A: OmniAgreementPublicKey
    Note over A: 型・長さ・作成時刻を検査
    A->>C: OmniAgreementPublicKey
    Note over C: 型・長さ・作成時刻を検査
    Note over C,A: transcript を構成し、X25519 の全 0 結果を拒否
    opt 発信側の auth_type が Sign
        C->>A: OmniCert(signature over Connected role and transcript)
        Note over A: 発信側の署名を検証
    end
    opt 受理側の auth_type が Sign
        A->>C: OmniCert(signature over Accepted role and transcript)
        Note over C: 受理側の署名を検証
    end
    Note over C,A: HKDF-SHA3-256 で鍵導出
    C->>A: KeyConfirmationMessage(Connected MAC)
    Note over A: MAC を検証
    A->>C: KeyConfirmationMessage(Accepted MAC)
    Note over C: MAC を検証
    Note over C,A: 各側は自分の MAC の送信完了と相手の MAC の検証後に stream を返す
```

profile と一時公開鍵の各段は、発信側が相手の同じ段の message を待たずに送信し、受理側はそれを受信・検査してから自分の message を送信する。
各側は profile の送受信と相手の profile の検査を終えてから一時公開鍵の段へ進み、一時公開鍵についても同じ条件を満たしてから cert の段へ進む。
cert の段では、発信側が `Sign` なら先に cert を送信し、受理側はそれを受信・検証してから、自分も `Sign` の場合に cert を送信する。
発信側が `None` なら、受理側は発信側の cert を待たず、自分が `Sign` の場合だけ cert を送信する。相手が `None` の側は相手の cert を待たず、双方が `None` なら cert の段を省く。
鍵確認は直列であり、発信側が MAC を送信し、受理側はそれを受信・検証してから自分の MAC を送信する。
各送信に対して相手が受信を行い、双方が同じ段で受信待ちになる状態も、双方が送信完了だけを待つ状態も作らないため、相互待ちによる deadlock は起きない。

各接続で新しい 32 byte のランダムな `session_id` と X25519 の一時鍵対を生成し、再利用しない。
profile の 4 つの algorithm flags はそれぞれ `1` のみを認め、X25519、HKDF、AES-256-GCM、SHA3-256 を使う。
署名は `OmniSignType::Ed25519_Sha3_256_Base64Url` のみを認める。

handshake の各 message は平文で、4 byte の little-endian 長さと RocketPack payload からなる frame に入る。
auth 層は長さを `max_frame_length` と照合してから payload を確保し、その message の末尾までだけを読む。
最後の鍵確認に続く byte 列を先読みせず、送信は message ごとに flush する。
これにより、handshake 後の暗号化した frame は下位 stream に未読のまま残る。

#### Transcript と署名

`||` は byte 列の連結、`C` は発信側、`A` は受理側を表す。
以下の `b"...\0"` は ASCII byte 列と末尾の 1 byte の NUL であり、整数と固定長の値に長さ prefix は付けない。
transcript `T` は、送受信の視点によらず、次の順で構成する。

```text
P_X = session_id[32]
      || auth_type(u32 LE)
      || key_exchange_algorithm_type_flags(u32 LE)
      || key_derivation_algorithm_type_flags(u32 LE)
      || cipher_algorithm_type_flags(u32 LE)
      || hash_algorithm_type_flags(u32 LE)
E_X = created_time.seconds(i64 BE)
      || agreement_algorithm_type(u32 LE)
      || public_key[32]
T   = b"OmniSecureStream/V1/transcript\0"
      || 0x01 || P_C || E_C
      || 0x02 || P_A || E_A
```

`auth_type` は `None=1`、`Sign=2`、`agreement_algorithm_type` は `X25519=2` とする。
profile の role は `T` の `0x01` と `0x02` に対応し、作成時刻は UTC の Unix 秒である。
wire の `export()` 全体や、受信後に選んだ値への置き換えは使わず、双方が送受信した意味的フィールドをそのまま含める。

role `r` の署名 preimage は `b"OmniSecureStream/V1/signature\0" || r || T` とする。
その SHA3-256 hash の 32 byte に `OmniSigner::sign` で署名し、相手は相手の role で同じ hash を計算して `OmniCert::verify` に渡す。
用途の接頭辞と role により、別用途の署名と反対向きの署名を流用できない。
記録した署名の再送は、受信側が接続ごとに新しい値を選ぶ前提で transcript の不一致として検出する。

#### 作成時刻と X25519 の検査

一時公開鍵の受信直後、署名の送信・検証と鍵導出に先立ち、algorithm が X25519、公開鍵が 32 byte であることを検査する。
同じ位置で `clock.now()` の UTC Unix 秒と `created_time.seconds` の差の絶対値を求め、`max_clock_skew` を超えれば拒否する。
過去と未来を同じ基準で検査し、境界値と同じ差は受理する。
差は `i128` で計算し、極端な `i64` の時刻でも overflow させない。
許容幅は整数秒に限り、端数を含む `Duration` は constructor で拒否する。0 秒も指定でき、既定値や検査を無効にする値は設けない。
生成側も同じ clock の Unix 秒を一時鍵の作成時刻にする。

X25519 の shared secret は、32 byte がすべて 0 なら `OmniAgreement::gen_secret` で拒否し、HKDF へ渡さない。
時計のずれが許容幅を超える接続は確立できず、許容幅内の再送の検出は新しい `session_id` と一時鍵に依存する。

#### HKDF と鍵確認

HKDF-SHA3-256 は X25519 の shared secret を IKM、`session_id_C || session_id_A` の 64 byte を salt、`b"OmniSecureStream/V1/keys\0" || SHA3-256(T)` を info として、152 byte を一度に展開する。
出力は次の順で分割する。

| byte 範囲（終端を含まない） | 用途 |
| --- | --- |
| `[0, 32)` | 発信側から受理側への frame の AES 鍵 |
| `[32, 44)` | 同方向の frame の初期 nonce |
| `[44, 76)` | 受理側から発信側への frame の AES 鍵 |
| `[76, 88)` | 同方向の frame の初期 nonce |
| `[88, 120)` | 発信側の鍵確認用 MAC 鍵 `K_C` |
| `[120, 152)` | 受理側の鍵確認用 MAC 鍵 `K_A` |

発信側は最初の frame 鍵と nonce を encode に、次の組を decode に使い、受理側は逆に使う。
鍵確認には専用の MAC 鍵を使い、frame の鍵と nonce は消費しない。
salt は一時鍵とは独立に選ぶ接続ごとの乱数から作り、info は用途と transcript に鍵を結び付ける。
salt と info の役割は [RFC 5869 §2–3](https://www.rfc-editor.org/rfc/rfc5869.html#section-2) に従う。

鍵確認の対象には `T` と、発信側・受理側の順に並べた署名公開鍵の slot `I_C`、`I_A` を含める。
自分の slot は自分が送った cert、相手の slot は検証済みの受信 cert から作り、相手から送られた slot の申告値は使わない。

```text
I_X (None) = 0x00
I_X (Sign) = 0x01 || cert.typ(u32 LE)
             || cert.public_key.len(u32 LE) || cert.public_key
M_r = b"OmniSecureStream/V1/confirmation\0" || r || T || I_C || I_A
MAC_r = HMAC-SHA3-256(K_r, M_r)
```

`cert.typ` は `Ed25519_Sha3_256_Base64Url=2` とし、公開鍵は cert に含まれる DER byte 列をそのまま使う。
`KeyConfirmationMessage` の schema は `@1 mac: bytes[32..=32];` の 1 field だけを持つ。
`KeyConfirmationMessage` はこの MAC の 32 byte を持ち、受信側は相手の role と対応する MAC 鍵で検証し、比較は定数時間で行う。
片方だけが署名する場合はその側だけが `Sign` slot、双方が `None` なら両 slot が `0x00` となる。
署名の公開鍵を中継者のものへ差し替えると、本人の送信 cert と相手の受信 cert で slot が食い違い、中継者が shared secret を知らない限り鍵確認を通せない。
双方が `None` の場合も鍵確認を交換するが、相手の身元の認証と能動的な中間者攻撃への保護は得られない。

#### 失敗と frame 層との境界

不正な role・形式・長さ・時刻、必須署名の欠落、署名や MAC の検証失敗、全 0 の shared secret は handshake の失敗とし、`auth()` と `OmniSecureStream::new` は `Err` を返す。
未対応の algorithm は `ErrorKind::UnsupportedType`、それ以外の検査違反は `ErrorKind::InvalidFormat` とし、I/O error と EOF も成功へ変換しない。
失敗通知や fallback は送らず、成功した stream と相手の cert を公開しない。
constructor は下位 stream を所有し、失敗時に保持している reader と writer を drop する。
呼び出し側も同じ接続を再利用せずに閉じ、handshake 全体の期限と同時数を管理する。

暗号化した frame の層は AES-256-GCM の鍵 32 byte と初期 nonce 12 byte の各方向 1 組だけを受け取り、transcript や cert を解釈しない。
[stream.rs](../modules/omnikit/src/service/connection/secure/stream.rs) の読み書きは、little-endian の 4 byte 長さと暗号文・16 byte の tag を扱い、平文を最大 64 KiB に分割する。
[encoder.rs](../modules/omnikit/src/service/connection/secure/encoder.rs) と [decoder.rs](../modules/omnikit/src/service/connection/secure/decoder.rs) は、初期 nonce から成功した frame ごとに [util.rs](../modules/omnikit/src/service/connection/secure/util.rs) の little-endian counter を進める。
新しい handshake も同じ長さの鍵と nonce を渡し、鍵確認で counter を進めないため、この層の変更を要しない。
`max_frame_length` は handshake の平文 frame にだけ適用し、暗号化した stream の固定 64 KiB の分割は変えない。

**AEAD の nonce は wire に載らず、送受信ごとに独立した counter を 1 message ごとに決定的に増分することでのみ一意性を保つ。**
この前提は、下位 stream が順序を保証する reliable な stream であることに依存する。
順序が保証されない下位 stream の上で使うと、両端の counter が同期せず decrypt が破綻する。

### 6.3 remoting

`OmniRemotingCaller`（呼び出し側）と `OmniRemotingListener`（受け側）は、1 つの下位 stream につき 1 回の関数呼び出しだけを扱う。
`OmniRemotingCaller::new` は stream を送受信半分に分割し、`HelloMessage { version, function_id }` を送って `OmniRemotingCaller` を返す。
呼び出し側はその `call_stream()` から `OmniRemotingStream` を取得し、送受信に使う。
`OmniRemotingListener::new` は同じ `HelloMessage` を検証し、`function_id()` として呼び出し先を露出したうえで、`listen_stream(callback)` に `OmniRemotingStream` を渡して callback を 1 回呼ぶ。
どの関数を呼ぶかを `function_id` から実際に分岐する処理は、このリモート層の外側（呼び出し側の実装）が担う。
1 回の呼び出しが 1 つの stream を専有するこの形は、1 つの secure connection 上で複数の呼び出しを同時に扱う用途には向かない（多重化の方式は §11.2 で保留している）。

### 6.4 model

`model/omni_hash.rs`、`omni_sign.rs`、`omni_agreement.rs` は、同名の `.rpf`（[modules/omnikit/rpfs](../modules/omnikit/rpfs)）から生成された `OmniHash`、`OmniSigner`、`OmniCert`、`OmniAgreement*` という wire 構造体へ、業務ロジックを直接 `impl` する。
別の wrapper 型は用意しない。

**型を参照する唯一の path は `generated` 側であり、`model` 側の module は再 export しない。**
`crate` 内外を問わず `generated::omni_sign::OmniSigner` のように参照する。
Rust の inherent impl と trait impl は module path ではなく型に紐づくため、`impl` の記述場所が `model` 側であっても、`generated` 側の path から掴んだ型にそのまま付いてくる。
この 3 つの `model` module は `impl` の置き場でしかなく、参照されないことを保証するために [model.rs](../modules/omnikit/src/model.rs) では非公開の `mod` として宣言する。
生成器が `src/generated` 以下を丸ごと差し替え、`// @generated by rocketpack-compiler` で始まらないファイルの上書きを拒否する（[codegen/rust.rs](../entrypoints/rocketpack-compiler/src/codegen/rust.rs) の `validate_managed_target`）ため、手書きの `impl` を生成物と同じ directory へ置くことはできない。
`.rpf` の package 名、`src/generated` 直下の module 名、`model` 直下の module 名は同一である。
`OmniHash::compute_hash` は SHA3-256、`OmniSigner::sign` と `OmniCert::verify` は Ed25519（`ed25519_dalek`）、`OmniAgreement::gen_secret` は X25519（`x25519_dalek`）を使う。
`OmniHash`、`OmniSigner`、`OmniCert` の `Display` は、[service/converter/omni_base.rs](../modules/omnikit/src/service/converter/omni_base.rs) の `OmniBase`（multibase 準拠の `f` が hex、`u` が base64url encoding）を経由する。
`OmniHash` は `sha3_256:u<base64url>`、`OmniSigner` と `OmniCert` は `{name}@u<base64url(sha3_256(public_key))>` のような人間可読な文字列を生成する。

`model/omni_addr.rs` の `OmniAddr` だけは対応する生成型を持たない、手書きの値型であり、`model` 直下で公開 module として残るのもこれだけである。
`tcp(ip4(...),port)` のような s 式風の記法を `nom` で parse し、endpoint を表す。
`OmniBase` や暗号鍵とは無関係であり、名前が似ている以外の関係はない。

## 7. yamux

`omnius-core-yamux` は、外部 crate `yamux`（stream 多重化そのものの実装）を、channel を介した actor 駆動の非同期 API として包む wrapper である。
frame や window、stream 状態の管理はすべて外部 crate 側が持ち、この crate は次の 2 点だけを足す。

- `YamuxConnection::new` が背後の driver task を `tokio::spawn` し、呼び出し側は poll を意識しない。
- `connect_stream` と `accept_stream` は `&self` であり、複数の task から並行に呼び出せる。
  実際の `yamux::Connection` への変更は driver task だけが行い、呼び出し側は channel 越しに依頼する。

`YamuxConnection::new<S>` は `S: AsyncRead + AsyncWrite + Send + Unpin + 'static` を要求する generic であり、下位 transport の型を問わない。
`close()` は shutdown 信号を送って driver task の終了を待つ、graceful な close である。
`Drop` は非 blocking な fallback であり、driver task を待たずに abort するため、graceful な close が必要な場合は `close().await` を明示的に呼ぶ必要がある。

`accept_backlog` は accept 待ち stream の上限であり、既定値は外部 crate `yamux` が ACK を待たずに開く stream 数の上限（`MAX_ACK_BACKLOG` = 256）と同じ 256 である。
対向が同じ実装であれば未 ACK の stream はこの数を超えないため、accept が遅れても backlog は溢れない。
backlog が溢れた場合、その stream は配送されずに drop され、外部 crate が対向へ RST を送る。
これは「受け入れられない stream は RST で拒否する」という yamux の規定に沿った拒否であり、対向は open の失敗として観測する。

## 8. cloud

`omnius-core-cloud` は、AWS（S3、Secrets Manager、SES、SQS）と GCP（Secret Manager）の SDK client を、小さな trait と `*Impl` 構造体の組として包む wrapper である。
各機能は同じ形（trait と、実 client を保持する `*Impl`、一部は `*Mock`）を独立に繰り返す。
共通の親 trait や marker は存在せず、AWS 側の `SecretsReader` と GCP 側の `SecretReader` のように、同じ役割でも trait 名と署名が別々に定義される。
両者を統一して cloud を切り替える呼び出し側コードを書くには、利用側で独自の統一 trait を用意する必要がある。

mock は呼び出しの記録と応答の再生に徹する。
入力は `Arc<parking_lot::Mutex<Vec<Input>>>` に記録され、戻り値のある method は `Arc<Mutex<VecDeque<T>>>` から `pop_front` する（尽きれば既定値を返す）。
実際のストレージやキューを模した状態は持たない。

GCP の `SecretReaderImpl::read_value` は呼び出しのたびに新しい API client を生成する。
AWS 側の各 `*Impl` は呼び出し側が構築して保持した `Client` を使い回す。

`aws` feature と `gcp` feature はそれぞれ独立して有効化でき、無効な側の依存 crate はコンパイルに含めない。

## 9. migration

`omnius-core-migration` は、`sqlx::migrate!` を使わず、PostgreSQL と SQLite それぞれに手書きした migration runner である。
history table への記録により、同じ migration を再実行しても再適用しない冪等性を持つ。

PostgreSQL（`postgres.rs`、`PostgresMigrator`）は次の手順で動く。

1. `_migrations`（適用済み file 名）と `_semaphores`（実行中を示す排他行）を作成する。
2. migration file を格納した directory を読み、file 名の辞書順を実行順として使う。
3. 未適用の file がなければ、lock を取らずに終了する。
4. `_semaphores` へ自分の `username` を primary key として insert する lock を取る。
   既に同じ `username` の行があれば insert は失敗する。
5. 未適用の file ごとに transaction を開き、`tokio_postgres::batch_execute` で file 全体を実行し、history へ記録して commit する。
   1 file が atomic な単位になる。
6. 成功しても失敗しても `_semaphores` の行を delete して lock を解放する。

**この lock は `pg_advisory_lock` ではなく、`_semaphores` table への insert と delete による自前の実装である。**
プロセスが lock 取得後に crash すると、`_semaphores` の行は TTL も自動 cleanup も持たないため、次回以降の migration がその `username` に対して lock を取れなくなるという前提の崩れ方をする。

SQLite（`sqlite.rs`、`SqliteMigrator`）は PostgreSQL と実行 engine から異なる。

- `sqlx::SqlitePool` を使う。
  PostgreSQL 側は `tokio_postgres` を直接使い、`sqlx` を経由しない。
- migration の供給元は directory ではなく、呼び出し側が組み立てて渡す `Vec<MigrationRequest>` である。
  順序は Vec の順そのままであり、file 名によるソートは行わない。
- `_semaphores` に相当する排他機構は持たない。
- 1 file 分の SQL を `';'` で単純分割してから 1 文ずつ実行する。
  文字列 literal やコメントの中の `;` は区別しないため、それらを含む SQL に対しては分割が意図と異なりうる。

[modules/testkit](../modules/testkit) を使う `tests/postgres.rs` は、Docker 上の実 PostgreSQL に対して migration の成功、構文 error での失敗、二重実行時の冪等性を検証する。
このテストは `stable-test` と `postgres` の両 feature を有効にした場合だけ build される。

## 10. testkit

`omnius-core-testkit` は、統合テストが使う Docker container を起動する helper である。
`containers::postgres::PostgresContainer` は、`testcontainers` crate で `postgres` image を起動し、起動完了を示す log message を待ってから接続文字列を組み立てる。

## 11. 設計判断

### 11.1 決定済み

#### omnikit wire 形式は flag-day で移行する

**決定**
RocketPack 生成コードの 1 始まり tag と enum-as-map を唯一の omnikit wire 形式とする。
旧ハンドコード形式の decode、旧署名の検証、旧形式で保存された値の読取は提供しない。

**理由**
workspace 内に旧形式を読む利用経路はなく、二重 decoder と version negotiation を維持するより、現行 schema を単一の正本に保つことを優先する。

**却下案**
旧形式の互換 decoder は rolling deployment や保存済み値の移行を可能にするが、tag と enum 表現の二重管理を恒久化するため採用しない。

#### 署名 preimage を意味的フィールドへ固定する

**決定**
secure auth は §6.2 の transcript を、発信側・受理側の順に双方の role、profile、一時公開鍵の意味的フィールドから構成する。
署名は用途の接頭辞と署名者の role を加えた preimage の hash を対象とし、同じ transcript の hash を HKDF の info に含める。
HKDF の salt は双方のランダムな session ID を発信側・受理側の順で連結する。
双方の署名公開鍵を含めた専用 MAC の鍵確認が成功してから handshake を完了する。
作成時刻の許容幅と相手の署名の必須指定は呼び出し側が決め、全 0 の X25519 結果を拒否する。
旧い V1 をこの手順で置き換え、frame の暗号化方式は維持する。

**理由**
双方の接続ごとの値と向きを署名することで、profile や一時公開鍵の書き換えと、過去の署名の再送を検出する。
公開鍵を含む鍵確認は、署名だけを別の署名者のものへ差し替える中継を検出し、導出した鍵を双方が保持することも確かめる。
用途の接頭辞は同じ署名鍵の別用途での署名の流用を防ぎ、時刻の検査は許容幅を超えた古い鍵を接続ごとの値に依存せず拒否する。
意味的フィールドを固定すると、wire の tag 順や serialization の変更だけでは preimage が変わらない。
暗号化した frame は鍵と nonce の受け渡しだけで使えるため、handshake の保証を加えるためにその層を作り直す必要がない。

**却下案**
wire export 全体を hash する方式は実装が短いが、serialization の変更だけで署名が無効になるため採用しない。
自分の profile と一時公開鍵だけを署名し、session ID の XOR を salt にする方式は、相手の値と接続の向きが署名に結び付かず、署名者を差し替える中継も検出できないため採用しない。
transcript だけを対象に鍵確認する方式は、同じ一時鍵を中継したまま署名者の公開鍵を差し替えられるため採用しない。
鍵確認を frame の AES 鍵と nonce で行う方式は、frame 層へ渡す counter の調整を必要とするため採用しない。

<a id="d-rpf-length-syntax"></a>
#### 有限な包含レンジを可変長型へ任意で後置する

**決定**
`string`、`bytes`、`Vec`、`Map` の各出現箇所は `[..=max]` または `[min..=max]` を後置できる。
範囲を書かない出現箇所は制約なしの可変長型となり、schema は長さを制限しない。
範囲を書く場合は包含かつ有限に限り、上限なしと排他的上限を認めない。
制約は nest 内でも各可変長型へ個別に置く。

**理由**
型に制約を結び付けると、外側と内側のどちらへ適用するかが構文上明確になり、範囲を書いた箇所の有限な最大値を compiler が一律に検査できる。
上限を定める根拠がない値にまで範囲を強いると、schema 作者は根拠のない数値を書くことになり、制約が書かれている事実そのものが contract として信用できなくなる。
括弧の有無で「上限を定めた」と「定めていない」が読み分けられるため、任意にしても曖昧さは生じない。

**却下案**
すべての出現箇所へ範囲を必須とする案は、上記の理由により採用しない。
`[..]` のような無制限を表す明示構文は、括弧の省略と同義の表記を増やすため採用しない。
`[min..]` のような下限のみの範囲は、両端を前提とする既存の runtime API と生成コードの分岐を増やす一方、必要な場合は `[min..=max]` で表現できるため採用しない。
生成器の設定で必須と任意を切り替える案は、同じ `.rpf` の可否が設定に依存し、schema が正本でなくなるため採用しない。
field attribute は nest 内の対象指定が複雑になるため採用しない。
名前付き generic 引数は型引数と設定値が混在するため採用しない。
bounded wrapper 型は生成 API を重くするため採用しない。

<a id="d-codec-boundary-enforcement"></a>
#### 標準 Rust 型を codec 境界で検査する

**決定**
生成 field の標準 Rust 型を維持し、encode と decode の両方で制約を検査する。

**理由**
既存の利用側 API を保ちながら、local に構築した不正値の送信と、wire から受け取った不正値の利用を同じ contract で拒否できる。

**却下案**
decode だけの検査は local の不正値を wire へ出力できるため採用しない。
構築時に不正値を表現できない wrapper 型は既存 field 型を変えるため採用しない。

<a id="d-declared-length-within-buffer"></a>
#### 宣言長が残り buffer に収まることを decoder が検査する

**決定**
`read_array` と `read_map` は、読み取った宣言長が入力の残り byte 数を超える場合に `UnexpectedEof` を返す。
この検査は schema の制約とは独立に働き、制約なしの可変長型にも、runtime API を直接呼ぶ利用側にも適用される。

**理由**
array と map の宣言長は wire 上で 8 byte まで取り得るため、9 byte の入力が `u64::MAX` 個の要素を宣言できる。
生成される decode は宣言長を容量として collection を確保するので、この検査がないと入力長に比例しない確保を外部から誘発できる。
要素は wire 上で最低 1 byte を占めるため、残り byte 数は要素数の正当な上限であり、正当な入力を一つも拒否しない。
`read_bytes` と `read_string` は payload を入力から切り出す時点で同じ検査を通るため、この決定は array と map だけに残っていた非対称を埋める。

**却下案**
宣言長を残り byte 数で切り詰める案は、不正な入力を error にせず短い値として受理するため採用しない。
生成コード側へ検査を置く案は、runtime API を直接使う呼び出し側を保護せず、同じ検査が生成物へ散らばるため採用しない。
message 全体の byte 数や総要素数の上限を runtime に持たせる案は、[長さ制約を各値へ限定する](#d-per-value-scope) と衝突するため採用しない。

<a id="d-timestamp-builtins"></a>
#### Timestamp を runtime 組み込み型として解決する

**決定**
非修飾かつ完全一致の `Timestamp64` と `Timestamp96` は予約済みの組み込み型とし、大小文字が異なる表記を alias として認めない。
struct、enum、type alias、import の短い名前が予約名と衝突する schema は compile error とする。
修飾付きの同名型は外部型として扱い、予約名ではない alias を付けた import を認める。
組み込み timestamp は field、enum payload、type alias、`Option`、`Vec`、`Map` の key と value、固定長 array の型位置で利用できる。
Rust generator はそれぞれ `omnius_core_rocketpack::primitive::Timestamp64` と `omnius_core_rocketpack::primitive::Timestamp96` へ解決し、生成する validate、encode、decode を runtime の `RocketPackStruct` に委譲する。
type alias の解決後に timestamp を含む field の default literal は schema compile 時に拒否する。
runtime wrapper は `Debug`、`Clone`、`PartialEq`、`Eq`、`PartialOrd`、`Ord` だけを実装要件とし、`Timestamp96.nanos` の意味と既存の wire encoding は変更しない。

**理由**
schema と生成 API が timestamp の精度を明示したまま、wire contract の正本を runtime の一か所に保てる。
予約名の衝突を生成前に拒否すると、組み込み型と利用者定義型のどちらへ解決されたかが一意になる。
runtime wrapper の比較 trait は生成型の derive と `BTreeMap` key の要件を満たすために限定する。

**却下案**
大小文字や snake case の別名は、同じ型を表す schema 表記を増やすため採用しない。
`DateTime<Utc>` への暗黙変換は schema 型と生成型の対応を不明瞭にするため採用しない。
generator 内での wire encoding の再実装は runtime と二重管理になるため採用しない。

<a id="d-bound-resolution"></a>
#### 境界値と type alias の解決範囲を限定する

**決定**
境界値は数値 literal または同じ package の符号なし整数定数とし、定数型は `u8`、`u16`、`u32`、`u64` に限定する。
type alias は宣言内の制約を引き継ぎ、利用側で範囲を後置できない。
宣言内に範囲を書かなかった alias も同じであり、利用側から初めて制約を与えることもできない。
`string` と `bytes` の default literal は schema compile 時に検査する。

**理由**
代表値を再利用できる一方で、式評価と制約合成を schema 言語へ持ち込まずに済む。
alias 名から contract が一意に定まるため、同じ alias が使用箇所ごとに違う長さを許すことがない。

**却下案**
imported const、演算式、負数、alias 利用側の制約上書きは、名前解決と優先順位を増やすため採用しない。
制約なしの alias にだけ利用側の制約を許す案は、alias を解決するまで可否が決まらず、制約合成を裏口から導入するため採用しない。

<a id="d-schema-version"></a>
#### Schema version 1 と wire encoding を維持する

**決定**
可変長制約の導入後も `.rpf` は `version 1;` を使い、wire encoding を変更しない。

**理由**
schema は一般公開されておらず、制約は既存 wire 値へ追加する検証 contract である。

**却下案**
`version 2;` への更新は、wire encoding を変えない内部 schema に移行分岐を増やすため採用しない。

<a id="d-per-value-scope"></a>
#### 長さ制約を各値へ限定する

**決定**
長さ制約は各可変長値へ適用し、message 全体の byte 数、総要素数、nest 深度を集計しない。

**理由**
field の意味上の上限と、transport または decoder 全体の resource budget は責務が異なる。

**却下案**
合計 size と nest depth の制約は、別の global policy を field 型へ混在させるため採用しない。

<a id="d-length-error-context"></a>
#### 制約違反を完全な schema path で報告する

**決定**
encoder と decoder は `LengthOutOfRange` を返し、`Request.tags[]`、`Request.attributes.key`、`Event.Upload.0` のような生成時に決まる schema path を `context` に持つ。

**理由**
呼び出し側とテストが error message の文字列解析に依存せず、nest 内の違反箇所を識別できる。

### 11.2 保留

#### omnikit の remoting 層を多重化する方式

**現状**
`OmniRemotingCaller` と `OmniRemotingListener` は、1 つの下位 stream につき 1 回の関数呼び出ししか扱わない。
`omnius-core-yamux` は 1 つの下位接続の上に複数 stream を多重化できるが、omnikit からは依存されておらず、結線されていない。

**なぜ今決めないか**
remoting 層を実際に使う呼び出し側の実装が workspace 内に存在せず、同時並行呼び出しの要否や、1 つの secure connection を複数呼び出しで再利用する要否が確定していない。

**決める条件**
omnikit の remoting を使う具体的な client または server の実装が必要になり、1 つの secure connection 上で複数呼び出しを扱う要否が定まったとき。

## 12. 現状と残作業

RocketPack の可変長型制約は任意であり、制約ありなしのどちらも parser、意味検査、Rust generator、runtime の境界検査へ反映されている。
`Timestamp64` と `Timestamp96` は Rust generator と生成例で利用できる。
RocketPack に関する §11.1 の決定済み contract に残作業はない。

§6.2 の handshake を実装している。
secure auth は双方の role、profile、一時公開鍵から transcript を構成し、署名と HKDF の info に含める。
相手の署名の必須指定と cert の取得に対応し、作成時刻の許容幅と全 0 の shared secret を検査する。
双方の署名公開鍵を含む鍵確認を交換してから stream を返す。
profile の role と鍵確認 message は schema と生成型にあり、HMAC-SHA3-256 には workspace と omnikit の直接依存 `hmac` を使う。

§6.2 の鍵と nonce は frame 層がそのまま使える。
auth 層は message の長さ分だけを読み、最後の鍵確認に続く暗号化した frame を先読みしない。
`stream.rs` の読み書き、encoder、decoder は従来のままである。

omnikit の secure connection 層と remoting 層はそれぞれ単体で動作するが、両者を結線して secure な経路上で remoting を行う実装は存在しない。
複数呼び出しを yamux で多重化する結線も存在しない（§11.2）。
`omnius-core-yamux` 自体は crate として完成している。

`omnius-core-cloud` の mock は `S3Client`、`SesSender`、`SqsSender` にはあるが、`SecretsReader`（aws）、`SqsReceiver`、`SecretReader`（gcp）にはない。

`omnius-core-testkit` は現状 `containers::postgres` だけを持ち、依存するのは modules/migration の統合テストだけである。

`omnius-core-image` は実装を持たないプレースホルダである。
`Cargo.toml` の依存関係（`kamadak-exif`、`tokio-postgres` など）は、EXIF metadata の抽出と PostgreSQL への永続化を意図していたと読めるが、コード上に実装はない（コード上に理由の記述はない）。

確認済みの不具合は [ISSUES.md](./ISSUES.md) に記録する。
