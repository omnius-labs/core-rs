# core-rs の既知の不具合

この文書は、core-rs のコードで確認した明確な不具合だけを扱う。
workspace 横断の設計判断と将来構想は [DESIGN.md](./DESIGN.md#11-設計判断) が扱う。
RocketPack compiler 固有の論点は [RocketPack compiler の設計](./design/rocketpack-compiler.md#10-設計判断) が扱う。

項目を外部 Issue に起票した場合は一覧の Issue 列へ番号を追記し、修正された場合は項目ごと削除する。
行番号は調査時点のスナップショットであるため、修正時に再確認する。

調査日: 2026-08-12

対象コミット: `d2d2faec5a03e5df724526f045ee47a14fac381d` に 2026-08-12 の対応差分を適用した working tree

| ID | 概要 | 深刻度 | Issue |
| --- | --- | --- | --- |
| [I-12](#i-12) | codegen が semantic の型体系を丸ごと再宣言する | 低 | |

I-12 は型追加時の保守性だけに影響し、現在の生成・codec の正確性を扱う項目とは独立している。

<a id="i-12"></a>
## I-12. codegen が semantic の型体系を丸ごと再宣言する

**深刻度: 低**（未顕在。現時点で生成物の不整合は確認していないため）

### 症状
新 builtin の追加に semantic と codegen の同時編集が必要で、型の写像が分岐する余地がある。

### 該当箇所
[`codegen/rust.rs#L65`](../entrypoints/rocketpack-compiler/src/codegen/rust.rs#L65), [`semantic.rs#L46`](../entrypoints/rocketpack-compiler/src/semantic.rs#L46)

### 原因
codegen が `BuiltinType` と `ResolvedType` を再宣言し、semantic の型を出力用型へ変換している。

### 影響
保守性の負債であり、現時点で動作不良は確認していない。

### 対応方針
semantic の `ResolvedType` を codegen が直接扱い、Rust path の解決だけを出力時に行う。

## ここに含めていないもの

| 論点 | 設計上の扱い |
| --- | --- |
| Remote dependency と lockfile | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
| 複数の Rust module root | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
