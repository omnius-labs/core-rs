## 1. このレビューについて

RocketPack compiler の書き直しと、それに伴う omnikit ワイヤメッセージの生成コードへの移行を差分レビューした。

### 1.1 文書間の責務分担

| 文書 | 受け持つもの |
| --- | --- |
| 本書 | 2026-08-10 にどの範囲をどの観点で見て何を判断したか |
| [DESIGN.md](../DESIGN.md) | なぜその形なのか。不変条件、採用理由、未決の論点 |
| [ISSUES.md](../ISSUES.md) | コードを読んで確認した明確な不具合。修正されるまで |

### 1.2 レビュー日と対象

- レビュー日: 2026-08-10
- レビューの型: 差分レビュー
- 対象コミット: base `95dd9438d39654ac064f27301bd220579e3a626a` (origin/main), head `54cbfdb05d559ba35590bc6b9f6af86d3bfb28fd` (feat/rocketpack-compiler-rewrite)
- レビュアー: Claude (z-ai/glm-5.2)、Claude (claude-opus-5)。前者が指摘を起草し、後者が発行前に全指摘を再検証した

## 2. 範囲と観点

### 2.1 見た範囲

差分レビューであり、base から head の間で変更されたファイルを実ファイルで読んだ。
主に対象としたのは、compiler 側の `entrypoints/rocketpack-compiler/src/` 配下（`parser.rs`, `parser/lexer.rs`, `semantic.rs`, `codegen/rust.rs`, `config.rs`, `error.rs`, `main.rs`）、runtime コーデックの `modules/rocketpack/src/` 配下（`rocket_pack_encoder.rs`, `rocket_pack_decoder.rs`, `rocket_pack_struct.rs`, `primitive/timestamp.rs`）、omnikit 移行の `modules/omnikit/src/` 配下（`generated/`, `model/`, `service/connection/secure/auth.rs`, `service/remoting/`）、および `modules/migration/src/` である。
生成済み `__rpf_*.rs` はスポットで読み、コンパイルとテストは実行した。
再検証では、各指摘を再現する最小の `.rpf` をスクラッチ領域に書いて実バイナリでコード生成し、生成物を `omnius-core-rocketpack` 依存のクレートで `cargo build` にかけた。
コーデックの指摘は、omnikit を依存に持つ実行プログラムでワイヤバイト列を組み立てて実際にデコードした。

### 2.2 見ていない範囲

- 差分の外のファイル。`modules/base`, `omnius-core-cloud`, `omnius-core-image`, `omnius-core-yamux`, `omnius-core-testkit` は今回変更されておらず、範囲外のため読んでいない。
- 5 つの生成済み `__rpf_*.rs` の全行。エンコードとデコードの対称性と制約の spot check はしたが、生成物全体の網羅的な読み比べはしていない。時間が他の観点に必要だったため。
- テストコードのカバレッジ。テストは実行して通過を確認したが、各指摘に対する対応テストの有無まで調べていない。観点に含めなかったため。
- 設計文書 `docs/DESIGN.md` と `docs/design/rocketpack-compiler.md` の内容の正確性。転記先の構造を確認するために見出しと保留章は読んだが、記述とコードの一致点検はしていない。別のレビューで扱う。

### 2.3 観点

- 生成コードの正確性: スキーマとして受理される型が、codegen でコンパイル可能な Rust になるか
- コーデックの入力検査: 敵対的・不正なワイヤ入力に対する decoder の境界とオーバーフローの保護
- ワイヤ形式の互換性: ハンドコードから生成コードへの移行で外部契約（ワイヤ形式、永続化形式、署名 preimage）が変わっていないか
- コンパイラの入力検証: スキーマの範囲外値（タグ、配列長）が拒否されるか
- 実行時の効率: 生成コードのエンコード経路の走査回数
- 保守性: codegen と semantic の型定義の重複

## 3. 総評

コンパイラ新設部に、スキーマとしては受理されるのに生成コードがコンパイル不能になる不具合（R-1, R-2, R-3）、実行時に必ず失敗する不具合（R-4）、無言で別の値を生成する不具合（R-5, R-6）がある。
いずれも parser と semantic で事前拒否するか、codegen で正しく処理するかのどちらかに決着させるまでマージできない。
ワイヤ形式の破壊的変更（R-11, R-12）は意図的マイグレーションと読めるが、互換・移行経路がなく fail-closed であり、後方互換性を保つか flag-day を許すかを決める必要がある。

## 4. 指摘

| ID | 概要 | 種別 | 確度 | 扱い |
| --- | --- | --- | --- | --- |
| [R-1](#r-1) | Option を Vec や Map 内に置くと生成コードが型不一致でコンパイル不能 | 不具合 | 高 | ISSUES.md に I-1 として起票 |
| [R-2](#r-2) | フィールド名 count や decoder が局所変数と衝突し生成コードがコンパイル不能 | 不具合 | 高 | ISSUES.md に I-2 として起票 |
| [R-3](#r-3) | sanitize_ident が衝突を検査せず type と type_ で重複メンバを生成 | 不具合 | 高 | ISSUES.md に I-3 として起票 |
| [R-4](#r-4) | u128 と i128 が実行時に必ず失敗するスタブを生成する | 不具合 | 高 | ISSUES.md に I-4 として起票 |
| [R-5](#r-5) | タグと配列長を u128 から無言トランケートする | 不具合 | 高 | ISSUES.md に I-5 として起票 |
| [R-6](#r-6) | Option フィールドの既定値が生成コードに反映されない | 不具合 | 高 | ISSUES.md に I-6 として起票 |
| [R-7](#r-7) | signed 正数デコードが iN 範囲超過を折り返して格納する | 不具合 | 高 | ISSUES.md に I-7 として起票 |
| [R-8](#r-8) | ワイヤ由来 count で Vec::with_capacity を呼びメモリ増幅する | 不具合 | 高 | ISSUES.md に I-8 として起票 |
| [R-9](#r-9) | skip_field が info 28 を 16 バイトと扱い read_raw_len と不一致 | 不具合 | 高 | ISSUES.md に I-9 として起票 |
| [R-10](#r-10) | 例示 config が base_dir: rpfs を宣言したまま rpfs を削除しコンパイル不能 | 不具合 | 高 | ISSUES.md に I-10 として起票 |
| [R-11](#r-11) | ワイヤ形式を 0 始まりタグから 1 始まりに変え互換経路がない | 設計論点 | 高 | DESIGN.md §11.2 保留「ワイヤ形式の後方互換性」に起票 |
| [R-12](#r-12) | 署名ハッシュの preimage を wire export 全体に変えた | 設計論点 | 高 | DESIGN.md §11.2 保留「署名ハッシュの preimage の安定性」に起票 |
| [R-13](#r-13) | pack が validate のために値ツリーをもう 1 パス走査する | 改善提案 | 高 | ISSUES.md に I-11 として起票（深刻度 低） |
| [R-14](#r-14) | codegen が semantic の型体系を丸ごと再宣言する | 改善提案 | 高 | ISSUES.md に I-12 として起票（深刻度 低） |
| [R-15](#r-15) | wip_tests が実経路を迂回した並行テスト harness として残っている | 改善提案 | 高 | ISSUES.md に I-13 として起票（深刻度 低） |

R-1 から R-6 までは、parser と semantic が受理する範囲と codegen が正しく扱える範囲の間に隙間があることに共通して由来する。
壊れ方は 3 通りに分かれる。
R-1 から R-3 は生成物がコンパイルを通らず、R-4 はコンパイルを通って実行時に必ず失敗し、R-5 と R-6 はコンパイルも実行も成功したうえで書いたスキーマと違う挙動になる。
最後の 2 つは失敗しないぶん気付きにくい。
R-7 から R-9 は runtime コーデックの入力検査で、負数側に比べて正数側にガードがない非対称と、未知フィールド読み飛ばしのヘッダ解釈の不一致に由来する。
R-11 と R-12 は同じ移行に由来し、ワイヤ形式と署名 preimage の両方で旧形式と互換がない。

<a id="r-1"></a>
### R-1. Option を Vec や Map の要素に置くと生成コードが型不一致でコンパイル不能

**種別: 不具合** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L672`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L672), [`semantic.rs#L358`](../../entrypoints/rocketpack-compiler/src/semantic.rs#L358)

**何が問題か**
`write_encode_value` と `write_decode_value` は `ResolvedType::Option(inner)` を透過的に剥がし、内側の型のエンコードとデコードに委譲する。
そのため `Vec<Option<u32>>` や `Map<String, Option<u32>>`、`Option<Option<u32>>`、`[Option<u32>; N]` をスキーマに書くと、decode 側は `Vec<Option<u32>>` に `u32` を push し、encode 側は `Option<u32>` を `write_u32` に渡し、両方で型不一致になる。

**根拠**
`semantic.rs` の `resolve_type_inner` は `Type::Option` の内側を再帰的に解決し、コンテナ内の Option を一切拒否しないことを読んだ。
codegen の `Option` 分岐が内側へ再帰して要素のエンコードを呼ぶことを読み、この組み合わせで `Vec<Option<u32>>` が E0308 になることをコンパイルで確認した。

**影響**
該当する型を使ったスキーマは生成コードのビルドで失敗する。
現時点の omnikit スキーマには該当する型がないため顕在化していない。

**扱い**
ISSUES.md に I-1 として起票。

<a id="r-2"></a>
### R-2. フィールド名 count や decoder が局所変数と衝突し生成コードがコンパイル不能

**種別: 不具合** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L718`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L718)

**何が問題か**
構造体 unpack は `let count = decoder.read_map()?;` を生成し、enum unpack は `let mut result: Option<Self> = None;` を生成する。
構造体でフィールド名に `count` か `decoder` を使うか、enum の record variant でフィールド名に `result` を使うと、同名の局所変数とシャドウイングし、match 内の代入が型エラーになる。

**根拠**
`@1 count: u32;` を持つ構造体を生成してコンパイルし、E0308 と、非可変の `count` に対する E0599 が出ることを確認した。
enum の record variant に `result` を置いて生成し、`result = Some(Self::A { .. })` が `Option<u32>` の局所変数への代入になって E0308 になることを確認した。
parser はこれらのフィールド名を拒否しないことを読んだ。

enum の record variant の `count` は衝突しない。
外側の `for _ in 0..count` は range を先に評価するため、variant の arm 内で `count` をシャドウイングしても影響しない。
これは生成してコンパイルが通ることで確認した。

**影響**
`@1 count: u32;` のようなフィールドを持つスキーマは生成コードのビルドで失敗する。
現時点の omnikit スキーマに該当フィールドがないため顕在化していない。

**扱い**
ISSUES.md に I-2 として起票。

<a id="r-3"></a>
### R-3. sanitize_ident が衝突を検査せず type と type_ で重複メンバを生成

**種別: 不具合** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L1903`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L1903)

**何が問題か**
`sanitize_ident` は Rust キーワードに `_` を付けるだけである。
`type` は `type_` になり、元からある `type_` と同じ識別子になる。
シンボルとパッケージ層には衝突検査があるが、フィールドとバリアント層にはないため、重複メンバを生成してコンパイル不能になる。

**根拠**
`sanitize_ident` の実装が `is_rust_keyword` だけで衝突を検査しないことと、`render_module_tree` がシンボルとパッケージのみ照合することを読んだ。
`type` と `type_` を同じ構造体に置くと両方 `type_` になることを確認した。

**影響**
`type` と `type_` のような名前の組を同じ構造体や enum に置いたスキーマは生成コードのビルドで失敗する。
現時点の omnikit スキーマに該当する組がないため顕在化していない。

**扱い**
ISSUES.md に I-3 として起票。

<a id="r-4"></a>
### R-4. u128 と i128 が実行時に必ず失敗するスタブを生成する

**種別: 不具合** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L655`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L655), [`semantic.rs#L569`](../../entrypoints/rocketpack-compiler/src/semantic.rs#L569)

**何が問題か**
semantic は `u128` と `i128` を正規 builtin として受理するが、codegen は `return Err(IoError(Unsupported, "u128 encode is not supported"))` と `Other("u128 decode is not supported")` のスタブを生成する。
`@1 id: u128;` はコンパイルに成功し、実行時に必ず失敗する。

**根拠**
semantic の `builtin_type` が `"u128" => Some(BuiltinType::U128)` を返すことと、codegen の `BuiltinType::U128` 分岐がエラーを返すスタブであることを読んだ。

**影響**
u128 か i128 を使ったフィールドのエンコードとデコードがすべて実行時に失敗する。
コンパイル時の拒否と固定幅実装のどちらにも決着していない。

**扱い**
ISSUES.md に I-4 として起票。

<a id="r-5"></a>
### R-5. タグと配列長を u128 から無言トランケートする

**種別: 不具合** / **確度: 高**

**該当箇所**
[`parser.rs#L625`](../../entrypoints/rocketpack-compiler/src/parser.rs#L625)

**何が問題か**
`expect_int_u32` と `expect_int_u32_spanned` と `expect_int_u64` は `Token::Int(n)` の `n: u128` を `n as u32` と `n as u64` で無言にトランケートする。
`@4294967296` はタグ 0 として生成され、`[u32; 18446744073709551617]` は `[u32; 1]` として生成される。
どちらもエラーにならず、書いたスキーマと違うワイヤ形式のコードが出る。

**根拠**
`expect_int_u32_spanned` が `Spanned::new(n as u32, ...)` とキャストしていることと、範囲外の値に対する検査がないことを読んだ。
`@4294967296` を単独で持つ構造体を生成し、`encoder.write_u64(0)` と `0 => {` が出ることを確認した。
`[u32; 18446744073709551617]` を生成し、`pub a: [u32; 1]` と `if __count_0 != 1` が出てコンパイルも通ることを確認した。

トランケートの結果が既存のタグと衝突する場合は semantic が拾う。
`@0` と `@4294967296` を並べると `duplicate field tag @0 in S` で拒否されることを確認した。
したがって重複した match arm は生成されず、この経路では unreachable pattern にならない。
検査がないのはトランケート単体であり、衝突しない限り最後まで通る。

**影響**
`2^32` 以上のタグと `2^64` 以上の配列長が、エラーにならないまま別の値になる。
生成されたエンコードとデコードは自己整合なので、スキーマを共有する別実装や別言語の生成物と突き合わせるまで食い違いに気付けない。

**扱い**
ISSUES.md に I-5 として起票。

<a id="r-6"></a>
### R-6. Option フィールドの既定値が生成コードに反映されない

**種別: 不具合** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L774`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L774)

**何が問題か**
`render_value_init` は resolved が `Option(_)` のとき早期 return し、フィールドの既定値を完全に無視する。
`@1 retry_count: Option<u32> = 3;` は、タグ 1 が欠落したとき `Some(3)` にならず `None` を返す。
必須欄の既定値は `unwrap_or` で効くが、Option 欄では効かず、前後で非対称である。

**根拠**
`render_value_init` の冒頭が `if matches!(resolved, ResolvedType::Option(_)) { return Ok(value_ident.to_string()); }` と早期 return し、default 引数を見ないことと、その後に `unwrap_or(default)` の経路があることを読んだ。
semantic が Option フィールドの既定値の組み合わせを受理することを読んだ。

**影響**
Option フィールドに既定値を書いても、ワイヤ上で欠落したときに既定値が入らない。
後方互換の既定値を意図したスキーマが効かない。

**扱い**
ISSUES.md に I-6 として起票。

<a id="r-7"></a>
### R-7. signed 正数デコードが iN 範囲超過を折り返して格納する

**種別: 不具合** / **確度: 高**

**該当箇所**
[`rocket_pack_decoder.rs#L295`](../../modules/rocketpack/src/rocket_pack_decoder.rs#L295)

**何が問題か**
`read_i64` の正数分岐 `(0, 27) => return Ok(u64::from_be_bytes(...) as i64)` は `u64 as i64` の無検証キャストである。
ワイヤ上の `2^63` を送ると `read_i64` はエラーを出さずに負数に折り返して構造体へ格納する。
`read_i8`, `read_i16`, `read_i32` も同じ形である。
負数側は top bit で範囲を検査しているのに、正数側にガードがない。

**根拠**
`read_i64` の正数分岐が `as i64` でキャストし、範囲を検査しないことと、負数分岐が `current_raw_byte() & 0x80` で検査することを読んだ。
encoder はこの範囲の正数を書かないが、外部や敵対的な CBOR プロデューサで再現できることを確認した。

**影響**
敵対的入力で i64 フィールドに任意の負数を入れられる。
データ破壊であり、現時点で omnikit が外部からの CBOR を受ける経路が結線されていないため未顕在である。

**扱い**
ISSUES.md に I-7 として起票。

<a id="r-8"></a>
### R-8. ワイヤ由来 count で Vec::with_capacity を呼びメモリ増幅する

**種別: 不具合** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L886`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L886)

**何が問題か**
非制約 `Vec<T>` の生成デコードはワイヤ由来の count を `Vec::with_capacity(count as usize)` にそのまま使う。
`read_array` の残りバイト検査は 1 要素 1 バイト以上を前提にすり抜ける範囲で、入力サイズの約 24 倍のメモリを一括確保できる。

**根拠**
生成される `Vec::with_capacity(count as usize)` と `read_array` の残りバイト検査を読み、`Vec<String>` で要素 1 バイトの空文字を並べたとき入力の約 24 倍を確保することを確認した。

**影響**
10 MiB の敵対バッファで約 240 MiB を確保できる。
外部入力経路が結線されていないため未顕在である。

**扱い**
ISSUES.md に I-8 として起票。

<a id="r-9"></a>
### R-9. skip_field が info 28 を 16 バイトと扱い read_raw_len と不一致

**種別: 不具合** / **確度: 高**

**該当箇所**
[`rocket_pack_decoder.rs#L490`](../../modules/rocketpack/src/rocket_pack_decoder.rs#L490)

**何が問題か**
`skip_field` は major 0 と 1 の additional info 28 を 16 バイトの payload `Some(16)` として扱う。
`read_raw_len` は info 28 以上を `None` とし、`read_u64` は `Mismatch` とするため、3 つの解釈が一致しない。
未知タグの値に `0x1C` を置くと `skip_field` だけ 16 バイト読み飛ばし、ストリーム位置がずれる。

**根拠**
`skip_field` の `28 => Some(16)` と、`read_raw_len` の info 28 以上の `None` 扱いを読み、解釈が一致しないことを確認した。
encoder は info 28 を生成しないため、敵対入力のみで発火する。

**影響**
正当なバッファでも、未知タグの値に不正ヘッダを置くと遠隔でデコード失敗を起こせる。
外部入力経路が結線されていないため未顕在である。

**扱い**
ISSUES.md に I-9 として起票。

<a id="r-10"></a>
### R-10. 例示 config が base_dir: rpfs を宣言したまま rpfs を削除しコンパイル不能

**種別: 不具合** / **確度: 高**

**該当箇所**
[`data/rocketpack.yaml#L5`](../../entrypoints/rocketpack-compiler/data/rocketpack.yaml#L5) (head 時点)

**何が問題か**
head の `data/rocketpack.yaml` は `base_dir: rpfs` を宣言するが、`rpfs` ディレクトリは削除済みである。
`cargo run -- compile ./data` が `source base_dir is not a directory` で失敗する。
修正は working tree での yaml 削除のみで、未コミットである。

**根拠**
head で `data/rocketpack.yaml` が `base_dir: rpfs` を持つことと、`rpfs` ディレクトリが存在しないことを確認した。
working tree では `data/rocketpack.yaml` が削除されていることを `git status` で確認した。

**影響**
このコミットを新しく clone した人が `compile ./data` で失敗する。
例示 config と rpfs の削除を同コミットにまとめていないため、途中状態が壊れている。

**扱い**
ISSUES.md に I-10 として起票。

<a id="r-11"></a>
### R-11. ワイヤ形式を 0 始まりタグから 1 始まりに変え互換経路がない

**種別: 設計論点** / **確度: 高**

**該当箇所**
[`__rpf_omni_secure_77c8b91862421dd1.rs#L173`](../../modules/omnikit/src/generated/omni_secure/__rpf_omni_secure_77c8b91862421dd1.rs#L173)

**何が問題か**
omnikit の全ワイヤメッセージが、旧ハンドコードの 0 始まりタグと u32・文字列エンコードの enum から、新生成コードの 1 始まりタグと enum-as-map に全面的に置き換わった。
`session_id` は `bytes[32..=32]` に厳格化された。
互換デコード経路やバージョン交渉がなく、旧ノードと旧形式の永続化データは復号不能になる。

**根拠**
生成された ProfileMessage のデコードが 1 始まりのタグと `read_bytes_bounded(32, 32)` を使うことと、削除された `message.rs` が 0 始まりタグと u32 の `auth_type` を使っていたことを読んだ。
旧形式で符号化した ProfileMessage のバイト列を新しい `unpack` に通し、`MismatchFieldType { position: 37, field_type: U8 }` で失敗することを確認した。
タグが 1 ずれた結果、旧タグ 1 の `auth_type`（major 0 の u32）を新タグ 1 の `session_id`（major 2 の bytes）として読もうとして落ちる。
欠落ではなく型不一致で落ちるため、既定値や欠落フィールドの補完では吸収できない。
互換経路の有無は設計判断であり、コードからは意図的に変えたと読めるが、移行方針の記述はない。

**影響**
ローリングデプロイや旧形式で保存されたデータとの共存が必要な場合、fail-closed で復号不能になる。
この repo 内では自己整合のためテストは通る。

**扱い**
DESIGN.md §11.2 保留「ワイヤ形式の後方互換性」に起票。

<a id="r-12"></a>
### R-12. 署名ハッシュの preimage を wire export 全体に変えた

**種別: 設計論点** / **確度: 高**

**該当箇所**
[`auth.rs#L179`](../../modules/omnikit/src/service/connection/secure/auth.rs#L179)

**何が問題か**
`gen_hash` の署名対象が、旧「フィールド単位の明示ハッシュ」から「ProfileMessage と OmniAgreementPublicKey のワイヤ `export()` 全バイト」に変わった。
署名検証が CBOR シリアライズのビット完全一致に依存するようになった。
将来のフィールド追加時も無言で署名不整合になる。

**根拠**
`gen_hash` が `hasher.update(profile_message.export()?)` と `hasher.update(agreement_public_key.export()?)` を呼ぶことと、旧コードが各フィールドのスカラを明示的にハッシュしていたことを読んだ。
同一ビルド内では自己整合だが、旧ビルドの署名と旧形式で保存された署名は構造的に検証不能なことを確認した。

**影響**
署名検証がシリアライズの安定性に依存する。
互換・移行経路の有無は設計判断である。

**扱い**
DESIGN.md §11.2 保留「署名ハッシュの preimage の安定性」に起票。

<a id="r-13"></a>
### R-13. pack が validate のために値ツリーをもう 1 パス走査する

**種別: 改善提案** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L489`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L489)

**何が問題か**
生成された `pack` は冒頭で `Self::validate(value)?` を呼ぶ。
Named 型フィールドは `has_length_constraints` が常に真とみなされるため、制約がなくてもサブツリー全体を検証する。
ネスト構造はノードごとに再検証され、エンコードが最低 2 パスになる。

**根拠**
`pack` の冒頭が `Self::validate(value)?` を呼ぶことと、`has_length_constraints` が `Named` と Timestamp で真を返すことを読んだ。

**影響**
エンコードコストが O(n と深さの積) に増える。
壊れてはいない。

**扱い**
ISSUES.md に I-11 として起票（深刻度 低）。

<a id="r-14"></a>
### R-14. codegen が semantic の型体系を丸ごと再宣言する

**種別: 改善提案** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L64`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L64), [`codegen/rust.rs#L971`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L971), [`codegen/rust.rs#L1598`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L1598)

**何が問題か**
codegen は `BuiltinType` と `ResolvedType` を `semantic.rs` から再宣言し、`convert_builtin` と `convert_resolved_type` で写像する。
builtin から Rust 型への文字列テーブルが `render_resolved_type` と `render_semantic_builtin` の 2 箇所に重複する。
新 builtin の追加に 6 箇所の同時編集が要る。

**根拠**
codegen の `BuiltinType` と `ResolvedType` が semantic と同じ要素を持つことと、2 つの render 関数が builtin から Rust 型への同じ写像を別々に持つことを読んだ。

**影響**
将来 2 つのテーブルが divergent すると、生成型が無言で食い違う。
壊れてはいない。

**扱い**
ISSUES.md に I-12 として起票（深刻度 低）。

<a id="r-15"></a>
### R-15. wip_tests が実経路を迂回した並行テスト harness として残っている

**種別: 改善提案** / **確度: 高**

**該当箇所**
[`codegen/rust.rs#L2122`](../../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L2122)

**何が問題か**
`mod wip_tests` は独自のテスト harness を持ち、`RustOptions` を直接構築して `render_rust_file` を呼ぶ。
そのため `validate_options` と `validate_required_mappings` による options 検証と、`publish_generated_files` の原子的置換・backup を迂回する。
隣の `mod tests` が `generate()` を end-to-end で駆動しているのに並行して存在する。

**根拠**
`mod wip_tests` が独自の `ParsedSource` と `GeneratedRustFile` を持ち、`render_sources` が `RustOptions` を直接構築して `publish_generated_files` を通さないことと、`mod tests` が `generate()` を通すことを読んだ。
`generate()` 側だけが `validate_options` を呼ぶことを読んだ。

semantic 検証は迂回していない。
`render_sources` は `ManifestGraph::load` と `SemanticGraph::build` を実際に通しており、この 2 つは実経路と同じである。

**影響**
options 検証と publish 経路の回帰検知に穴が残る。
壊れてはいない。

**扱い**
ISSUES.md に I-13 として起票（深刻度 低）。

## 5. 確かめて問題がなかったこと

- decoder のスライス境界と `pos + len` のオーバーフロー保護は健全か。`read_raw_bytes` と `read_raw_fixed_bytes` が `remaining() < len` を検査し、`skip_field` の残り計算が `checked_add` と `checked_mul` を使うことから、ワイヤ由来長によるパニックはないと判断した。
- 生成コードの encode と decode の対称性。両方向が同一スキーマから tag 順で生成され、未知タグは `_ => skip_field()` で飛ばすことから、フィールド順序の不一致や孤立 type id はないと判断した。
- `EmptyRocketMessage` と `rocketpack::prelude` の削除。grep で残存参照がないことを確認し、外部呼び出し元がないと判断した。
- `strum` と `enumflags2` の削除。grep で残存使用がないことを確認した。
- migration の `&mut self` 変更。workspace 内に外部呼び出し元がなく、テストが更新済みであることを確認した。
- ワークスペース全体のビルドとテスト。compiler 34, rocketpack 25, omnikit 6 がすべて通ることを実行して確認した。omnikit にはこのほかに 3 件の ignored があり、これらは実行していない。

## 6. その後

- 2026-08-10: 発行。R-1 から R-10 と R-13 から R-15 を ISSUES.md に I-1 から I-13 として起票。R-11 と R-12 を DESIGN.md §11.2 保留に起票。
- 2026-08-11: R-10 を修正。`entrypoints/rocketpack-compiler/data/rocketpack.yaml` を削除し、例示 config が消えた `rpfs` を指す状態を解消した。ISSUES.md の I-10 は削除済み。compiler の例示は `entrypoints/rocketpack-compiled-example/` の 3 project が引き継ぐ。
