# core-rs の既知の不具合

この文書は、core-rs のコードで確認した明確な不具合だけを扱う。
workspace 横断の設計判断と将来構想は [DESIGN.md](./DESIGN.md#11-設計判断) が扱う。
RocketPack compiler 固有の論点は [RocketPack compiler の設計](./design/rocketpack-compiler.md#10-設計判断) が扱う。

項目を外部 Issue に起票した場合は一覧の Issue 列へ番号を追記し、修正された場合は項目ごと削除する。
行番号は調査時点のスナップショットであるため、修正時に再確認する。

調査日: 2026-08-09

対象コミット: `95dd9438d39654ac064f27301bd220579e3a626a` に本作業の未コミット差分を適用した working tree

| ID | 概要 | 深刻度 | Issue |
| --- | --- | --- | --- |

現在、確認済みの未修正不具合はない。

## ここに含めていないもの

| 論点 | 設計上の扱い |
| --- | --- |
| Remote dependency と lockfile | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
| 複数の Rust module root | [RocketPack compiler の設計にある保留事項](./design/rocketpack-compiler.md#102-保留) |
