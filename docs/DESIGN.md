# RocketPack 設計

## 1. このドキュメントについて

本書は、`core-rs` にある RocketPack の Rust runtime、`.rpf` compiler、生成コードの責務と不変条件を扱う。
対象読者は、schema 言語、codec、Rust generator を変更する実装者と reviewer である。

### 1.1 文書間の責務分担

| 文書または正本 | 受け持つもの |
| --- | --- |
| 本書 | RocketPack の責務境界、不変条件、設計判断、実装状況 |
| `ISSUES.md`（未作成） | コードで確認した明確な不具合が生じた場合の作業用一覧 |
| [entrypoints/rocketpack-compiler](../entrypoints/rocketpack-compiler) | `.rpf` の構文、意味検査、Rust code generation の実装 |
| [modules/rocketpack](../modules/rocketpack) | wire codec と生成コードが利用する runtime API の実装 |
| [entrypoints/rocketpack-compiled-example/rpfs](../entrypoints/rocketpack-compiled-example/rpfs) | 生成例が利用する `.rpf` schema の正本 |

### 1.2 本書の時制について

本文は合意済みの完成形を現在形で記述する。
実装状況と残作業は §7 に集約する。
いま何が動くのかを知りたい場合は §7 を先に読む。

## 2. RocketPack とは

RocketPack は、`.rpf` schema から Rust のデータ型と wire codec を生成する仕組みである。
schema は wire 上の field tag と型を定め、runtime は CBOR 互換の長さ表現を読み書きする。

### 2.1 スコープ外

- C# と Swift の generator は対象外であるため、本書はそれらの生成 API と検証方式を定めない。
- message 全体の byte 数、総要素数、nest 深度は codec または transport の別制約とし、field ごとの長さ制約へ集約しない。
- `core-rs` 以外にある `.rpf` の移行は各 repository が所有するため、本書は repository 間の同期機構を持たない。

## 3. 全体構成

### 3.1 コンポーネントとディレクトリ対応表

| パス | package | 役割 |
| --- | --- | --- |
| [modules/rocketpack](../modules/rocketpack) | `omnius-core-rocketpack` | encoder、decoder、`RocketPackStruct` の runtime contract |
| [entrypoints/rocketpack-compiler](../entrypoints/rocketpack-compiler) | `omnius-core-rocketpack-compiler` | `.rpf` の parse、意味検査、Rust code generation |
| [entrypoints/rocketpack-compiled-example/rpfs](../entrypoints/rocketpack-compiled-example/rpfs) | 該当なし | 生成例の schema 正本 |
| [entrypoints/rocketpack-compiled-example/rust/gen](../entrypoints/rocketpack-compiled-example/rust/gen) | `rocketpack-compiled-example` | compiler が出力する Rust code であり、手で編集しない |

### 3.2 処理の流れ

```mermaid
flowchart LR
    RPF[.rpf schema] --> Parser[parser and semantic validation]
    Parser --> Generator[Rust generator]
    Generator --> Generated[generated Rust types and codec]
    Generated --> Runtime[RocketPack runtime]
    Runtime --> Wire[wire bytes]
```

schema の制約は compiler が一度解決し、生成コードと runtime の境界検査へ反映する。

## 4. 中心概念

### 4.1 Schema

**Schema** は `.rpf` に記述した package、型、field tag、定数、制約の集合である。
`.rpf` が正本であり、生成された Rust code は schema から再生成できる派生成果物である。

### 4.2 可変長型の制約

**可変長型の制約** は、`string`、`bytes`、`Vec`、`Map` の各値が取り得る長さの包含範囲である。
`string` と `bytes` は byte 数、`Vec` は要素数、`Map` は wire 上の entry 数を長さとする。
`Option` は値の有無だけを表し、固定長 array は外側の長さが型で決まるため、それぞれ内包する可変長型だけが制約を持つ。

## 5. 制約の検査境界

### 5.1 Schema compile 時

compiler は有限な上限、境界値の解決、範囲の順序、type alias 内の制約、default literal を生成前に検査する。
どの schema にも不正があれば、生成物を書き出す前に失敗する。

### 5.2 Codec 実行時

生成される field は `String`、`Vec`、`BTreeMap` などの標準 Rust 型を保つ。
encode は length prefix を書く前に検査し、decode は collection の確保、反復、payload の所有化より前に宣言長を検査する。
制約違反は schema path と実際の長さを持つ専用 error として返す。

## 6. 設計判断

### 6.1 決定済み

<a id="d-rpf-length-syntax"></a>
#### 有限な包含レンジを可変長型へ後置する

**決定**
`string`、`bytes`、`Vec`、`Map` の各出現箇所は `[..=max]` または `[min..=max]` を必須とし、上限なしと排他的上限を認めない。
制約は nest 内でも各可変長型へ個別に置く。

**理由**
型に制約を結び付けると、外側と内側のどちらへ適用するかが構文上明確になり、有限な最大値を compiler が一律に検査できる。

**却下案**
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
type alias は宣言内の制約を引き継ぎ、利用側で再制約しない。
`string` と `bytes` の default literal は schema compile 時に検査する。

**理由**
代表値を再利用できる一方で、式評価と制約合成を schema 言語へ持ち込まずに済む。

**却下案**
imported const、演算式、負数、alias 利用側の制約上書きは、名前解決と優先順位を増やすため採用しない。

<a id="d-schema-version"></a>
#### Schema version 1 と wire encoding を維持する

**決定**
可変長制約の必須化後も `.rpf` は `version 1;` を使い、wire encoding を変更しない。

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

### 6.2 保留

現在、可変長型の制約について保留している設計論点はない。

## 7. 現状と残作業

可変長型の制約は parser、意味検査、Rust generator、runtime の境界検査へ反映されている。
`Timestamp64` と `Timestamp96` は Rust generator と生成例で利用できる。
§6.1 の決定済み contract に残作業はない。

確認済みの不具合を記録する `ISSUES.md` は現時点で存在しない。
