# core-rs の既知の不具合

この文書は、core-rs のコードで確認した明確な不具合だけを扱う。
workspace 横断の設計判断と将来構想は [DESIGN.md](./DESIGN.md#11-設計判断) が扱う。
RocketPack compiler 固有の論点は [RocketPack compiler の設計](./design/rocketpack-compiler.md#10-設計判断) が扱う。

項目を外部 Issue に起票した場合は一覧の Issue 列へ番号を追記し、修正された場合は項目ごと削除する。
行番号は調査時点のスナップショットであるため、修正時に再確認する。

調査日: 2026-08-10

対象コミット: `95dd9438d39654ac064f27301bd220579e3a626a` と `54cbfdb05d559ba35590bc6b9f6af86d3bfb28fd` の間の差分

| ID | 概要 | 深刻度 | Issue |
| --- | --- | --- | --- |
| [I-1](#i-1) | Option を Vec や Map の要素に置くと生成コードが型不一致でコンパイル不能 | 高（未顕在） | |
| [I-2](#i-2) | フィールド名 count や decoder が局所変数と衝突し生成コードがコンパイル不能 | 高（未顕在） | |
| [I-3](#i-3) | sanitize_ident が衝突を検査せず type と type_ で重複メンバを生成 | 高（未顕在） | |
| [I-4](#i-4) | u128 と i128 が実行時に必ず失敗するスタブを生成する | 高 | |
| [I-5](#i-5) | タグと配列長を u128 から無言トランケートする | 高 | |
| [I-6](#i-6) | Option フィールドの既定値が生成コードに反映されない | 高 | |
| [I-7](#i-7) | signed 正数デコードが iN 範囲超過を折り返して格納する | 高（未顕在） | |
| [I-8](#i-8) | ワイヤ由来 count で Vec::with_capacity を呼びメモリ増幅する | 高（未顕在） | |
| [I-9](#i-9) | skip_field が info 28 を 16 バイトと扱い read_raw_len と不一致 | 高（未顕在） | |
| [I-10](#i-10) | 例示 config が base_dir: rpfs を宣言したまま rpfs を削除しコンパイル不能 | 高 | |
| [I-11](#i-11) | pack が validate のために値ツリーをもう 1 パス走査する | 低 | |
| [I-12](#i-12) | codegen が semantic の型体系を丸ごと再宣言する | 低 | |
| [I-13](#i-13) | wip_tests が実経路を迂回した並行テスト harness として残っている | 低 | |

I-1 から I-6 までは RocketPack compiler のコード生成に共通する。
いずれもスキーマとしては受理されるが生成時に壊れるか、コンパイルは通るが実行時に必ず失敗する。
I-7 から I-9 は runtime コーデックの入力検査で、外部入力経路が未結線のため未顕在である。

<a id="i-1"></a>
## I-1. Option を Vec や Map の要素に置くと生成コードが型不一致でコンパイル不能

**深刻度: 高**（未顕在。現時点の omnikit スキーマに該当する型がないため）

### 症状
`Vec<Option<u32>>` や `Map<String, Option<u32>>`、`Option<Option<u32>>`、`[Option<u32>; N]` をスキーマに書くと、生成された Rust が E0308 でビルドに失敗する。

### 該当箇所
[`codegen/rust.rs#L672`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L672), [`semantic.rs#L358`](../entrypoints/rocketpack-compiler/src/semantic.rs#L358)

### 原因
`write_encode_value` と `write_decode_value` は `ResolvedType::Option(inner)` を透過的に剥がし、内側の型のエンコードとデコードに委譲する。
`semantic.rs` の `resolve_type_inner` はコンテナ内の Option を一切拒否しない。
そのため decode 側は `Vec<Option<u32>>` に `u32` を push し、encode 側は `Option<u32>` を `write_u32` に渡す。

### 影響
該当する型を使ったスキーマは生成コードのビルドで失敗する。

### 対応方針
parser と semantic でコンテナ内の Option を事前拒否するか、codegen で Option を正しく処理するかのどちらかに決着する。

<a id="i-2"></a>
## I-2. フィールド名 count や decoder が局所変数と衝突し生成コードがコンパイル不能

**深刻度: 高**（未顕在。現時点の omnikit スキーマに該当フィールドがないため）

### 症状
構造体のフィールド名に `count` か `decoder` を使うか、enum の record variant のフィールド名に `result` を使うと、生成された Rust が型エラーでビルドに失敗する。

### 該当箇所
[`codegen/rust.rs#L718`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L718)

### 原因
構造体 unpack は `let count = decoder.read_map()?;` を生成し、enum unpack は `let mut result: Option<Self> = None;` を生成する。
これらと同名のフィールドがあるとシャドウイングし、match 内の代入が型エラーになる。
parser はこれらのフィールド名を拒否しない。
enum の record variant の `count` は、外側の `for _ in 0..count` が range を先に評価するため衝突しない。

### 影響
該当するフィールド名を持つスキーマは生成コードのビルドで失敗する。

### 対応方針
生成する局所変数名を予約語と衝突しない接頭辞付きの名前にするか、フィールド名の衝突を semantic で検査する。

<a id="i-3"></a>
## I-3. sanitize_ident が衝突を検査せず type と type_ で重複メンバを生成

**深刻度: 高**（未顕在。現時点の omnikit スキーマに該当する組がないため）

### 症状
`type` と `type_` のような名前の組を同じ構造体や enum に置くと、両方 `type_` になり重複メンバでビルドに失敗する。

### 該当箇所
[`codegen/rust.rs#L1903`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L1903)

### 原因
`sanitize_ident` は Rust キーワードに `_` を付けるだけで衝突を検査しない。
シンボルとパッケージ層には衝突検査があるが、フィールドとバリアント層にはない。

### 影響
該当する名前の組を持つスキーマは生成コードのビルドで失敗する。

### 対応方針
フィールドとバリアント層で衝突検査を追加するか、sanitize 後の識別子の衝突を semantic で検査する。

<a id="i-4"></a>
## I-4. u128 と i128 が実行時に必ず失敗するスタブを生成する

**深刻度: 高**

### 症状
`@1 id: u128;` のようなフィールドはコンパイルに成功するが、エンコードとデコードが実行時に必ず Unsupported と Other で失敗する。

### 該当箇所
[`codegen/rust.rs#L655`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L655), [`semantic.rs#L569`](../entrypoints/rocketpack-compiler/src/semantic.rs#L569)

### 原因
semantic は `u128` と `i128` を正規 builtin として受理するが、codegen は `return Err(IoError(Unsupported, "u128 encode is not supported"))` と `Other("u128 decode is not supported")` のスタブを生成する。

### 影響
u128 か i128 を使ったフィールドのエンコードとデコードがすべて実行時に失敗する。

### 対応方針
コンパイル時の拒否と固定幅実装のどちらかに決着させる。

<a id="i-5"></a>
## I-5. タグと配列長を u128 から無言トランケートする

**深刻度: 高**

### 症状
`@4294967296` はタグ 0 として、`[u32; 18446744073709551617]` は `[u32; 1]` として、エラーなく生成される。
書いたスキーマと違うワイヤ形式のコードが出る。

### 該当箇所
[`parser.rs#L625`](../entrypoints/rocketpack-compiler/src/parser.rs#L625)

### 原因
`expect_int_u32` と `expect_int_u32_spanned` と `expect_int_u64` は `Token::Int(n)` の `n: u128` を `n as u32` と `n as u64` で無言にトランケートし、範囲外の値に対する検査がない。
トランケートの結果が既存のタグと衝突した場合だけ semantic が `duplicate field tag` で拾うため、衝突しない限り最後まで通る。

### 影響
`2^32` 以上のタグと `2^64` 以上の配列長が、エラーにならないまま別の値になる。
生成されたエンコードとデコードは自己整合なので、スキーマを共有する別実装や別言語の生成物と突き合わせるまで食い違いに気付けない。

### 対応方針
キャスト前に範囲検査を追加し、超過した場合は parser error を返す。

<a id="i-6"></a>
## I-6. Option フィールドの既定値が生成コードに反映されない

**深刻度: 高**

### 症状
`@1 retry_count: Option<u32> = 3;` のように Option フィールドに既定値を書いても、ワイヤ上でタグが欠落したときに `Some(3)` にならず `None` を返す。

### 該当箇所
[`codegen/rust.rs#L774`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L774)

### 原因
`render_value_init` は resolved が `Option(_)` のとき早期 return し、フィールドの既定値を完全に無視する。
必須欄の既定値は `unwrap_or` で効くが、Option 欄では効かず非対称である。
semantic はこの組み合わせを受理する。

### 影響
後方互換の既定値を意図したスキーマが効かない。

### 対応方針
Option フィールドの既定値を生成コードに反映するか、semantic でこの組み合わせを拒否するかのどちらかに決着する。

<a id="i-7"></a>
## I-7. signed 正数デコードが iN 範囲超過を折り返して格納する

**深刻度: 高**（未顕在。外部入力経路が未結線のため。外部から誘発可能）

### 症状
敵対的 CBOR プロデューサが i64 フィールドに `2^63` を送ると、`read_i64` はエラーを出さずに負数に折り返して構造体へ格納する。

### 該当箇所
[`rocket_pack_decoder.rs#L295`](../modules/rocketpack/src/rocket_pack_decoder.rs#L295)

### 原因
`read_i64` の正数分岐 `(0, 27) => return Ok(u64::from_be_bytes(...) as i64)` は `u64 as i64` の無検証キャストである。
`read_i8`, `read_i16`, `read_i32` も同じ形である。
負数側は top bit で範囲を検査しているのに正数側にガードがない。

### 影響
敵対的入力で i64 フィールドに任意の負数を入れられる。

### 対応方針
正数分岐でも `iN::MAX` を超える値を検査してエラーにする。

<a id="i-8"></a>
## I-8. ワイヤ由来 count で Vec::with_capacity を呼びメモリ増幅する

**深刻度: 高**（未顕在。外部入力経路が未結線のため。外部から誘発可能）

### 症状
`Vec<String>` フィールドに要素 1 バイトの空文字を多数並べた入力を送ると、入力サイズの約 24 倍のメモリを一括確保できる。10 MiB の入力で約 240 MiB になる。

### 該当箇所
[`codegen/rust.rs#L886`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L886)

### 原因
非制約 `Vec<T>` の生成デコードはワイヤ由来の count を `Vec::with_capacity(count as usize)` にそのまま使う。
`read_array` の残りバイト検査は 1 要素 1 バイト以上を前提にすり抜ける範囲で許容する。

### 影響
敵対的入力でメモリ増幅 DoS ができる。

### 対応方針
count を `min(count, remaining)` で制限するか、制約付きの `read_array_bounded` を非制約 Vec にも使う。

<a id="i-9"></a>
## I-9. skip_field が info 28 を 16 バイトと扱い read_raw_len と不一致

**深刻度: 高**（未顕在。外部入力経路が未結線のため。外部から誘発可能）

### 症状
未知タグの値に不正ヘッダ `0x1C` を置くと `skip_field` だけ 16 バイト読み飛ばし、ストリーム位置がずれて後続フィールドをずれたオフセットから読む。正当なバッファでも遠隔でデコード失敗を起こせる。

### 該当箇所
[`rocket_pack_decoder.rs#L490`](../modules/rocketpack/src/rocket_pack_decoder.rs#L490)

### 原因
`skip_field` は major 0 と 1 の additional info 28 を 16 バイトの payload `Some(16)` として扱うが、`read_raw_len` は info 28 以上を `None` とし、`read_u64` は `Mismatch` とするため、3 つの解釈が一致しない。
encoder は info 28 を生成しないため敵対入力のみで発火する。

### 影響
敵対的入力でデコード失敗を起こせる。

### 対応方針
`skip_field` の info 28 の扱いを `read_raw_len` と一致させる。

<a id="i-10"></a>
## I-10. 例示 config が base_dir: rpfs を宣言したまま rpfs を削除しコンパイル不能

**深刻度: 高**

### 症状
head の `data/rocketpack.yaml` は `base_dir: rpfs` を宣言するが `rpfs` ディレクトリは削除済みで、`cargo run -- compile ./data` が `source base_dir is not a directory` で失敗する。

### 該当箇所
[`data/rocketpack.yaml#L5`](../entrypoints/rocketpack-compiler/data/rocketpack.yaml#L5) (head 時点)

### 原因
例示 config と rpfs の削除を同コミットにまとめていない。
修正は working tree での yaml 削除のみで未コミットである。

### 影響
このコミットを新しく clone した人が `compile ./data` で失敗する。

### 対応方針
例示 config と rpfs の削除を同コミットにまとめる。

<a id="i-11"></a>
## I-11. pack が validate のために値ツリーをもう 1 パス走査する

**深刻度: 低**

### 症状
エンコードコストが最低 2 パスになり、ネスト構造ではノードごとに再検証される。

### 該当箇所
[`codegen/rust.rs#L489`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L489)

### 原因
生成された `pack` は冒頭で `Self::validate(value)?` を呼ぶ。
Named 型フィールドは `has_length_constraints` が常に真とみなされるため、制約がなくてもサブツリー全体を検証する。

### 影響
エンコードコストが O(n と深さの積) に増える。
壊れてはいない。

### 対応方針
validate を message 信頼境界のみに寄せるか、インラインの bounds check を単一エンコードパスに折り込む。

<a id="i-12"></a>
## I-12. codegen が semantic の型体系を丸ごと再宣言する

**深刻度: 低**

### 症状
新 builtin の追加に 6 箇所の同時編集が要り、将来 2 つのテーブルが divergent すると生成型が無言で食い違う。

### 該当箇所
[`codegen/rust.rs#L64`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L64), [`codegen/rust.rs#L971`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L971), [`codegen/rust.rs#L1598`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L1598)

### 原因
codegen は `BuiltinType` と `ResolvedType` を `semantic.rs` から再宣言し、`convert_builtin` と `convert_resolved_type` で写像する。
builtin から Rust 型への文字列テーブルが `render_resolved_type` と `render_semantic_builtin` の 2 箇所に重複する。

### 影響
保守性の負債であり、壊れてはいない。

### 対応方針
semantic の `ResolvedType` を直接扱い、テーブルを 1 つに統合する。

<a id="i-13"></a>
## I-13. wip_tests が実経路を迂回した並行テスト harness として残っている

**深刻度: 低**

### 症状
options 検証と publish 経路の回帰検知に穴が残る。

### 該当箇所
[`codegen/rust.rs#L2122`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L2122)

### 原因
`mod wip_tests` は独自のテスト harness を持ち、`RustOptions` を直接構築して `render_rust_file` を呼ぶため、`validate_options` と `validate_required_mappings` による options 検証と、`publish_generated_files` の原子的置換・backup を迂回する。
`ManifestGraph::load` と `SemanticGraph::build` は通しているため、semantic 検証は迂回していない。
隣の `mod tests` が `generate()` を end-to-end で駆動しているのに並行して存在する。

### 影響
壊れてはいないが、options 検証と publish を迂回したテストが偽の安心を与える。

### 対応方針
`wip_tests` のアサーションを `mod tests` の `generate()` 経路に折り込み、並行 harness を削除する。

## ここに含めていないもの

| 論点 | 設計上の扱い |
| --- | --- |
| Remote dependency と lockfile | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
| 複数の Rust module root | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
| ワイヤ形式の後方互換性 | [DESIGN.md の保留事項](./DESIGN.md#112-保留) |
| 署名ハッシュの preimage の安定性 | [DESIGN.md の保留事項](./DESIGN.md#112-保留) |
