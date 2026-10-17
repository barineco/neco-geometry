# 幾何演算の検証

この検証では、neco-geometry の公開 API が返した値を観測として記録し、Lean で書いた検査器が入力から計算した値と照合します。この文書の手順に従うと、領域多項式 ( 代数的領域を定める符号条件に現れる、変数 x, y, z の多項式 ) の正規化から曲面上の点の評価までの 6 種類の演算について、観測が検査を通ることと、改変した観測や不正な入力を検査器が拒否することを、手元で再現できます。
観測は Rust の生成器 ( producer ) が観測バッチへ書き出し、Lean の検査器 ( checker ) が観測ごとに判定を返します。
観測バッチの形式と判定の意味は、[`protocol/README.md`](protocol/README.md) の定義どおりです。

領域多項式の正規化と、曲面上の点の包含区間の評価では、有理数の座標とパラメータについて、検査器の計算の正しさを Lean の健全性定理 ( soundness theorem ) で証明しています。前者の定理は、正規化の前と後で多項式の値が等しいことを示します。後者の定理は、検査器が計算した区間が真の座標を含むことを示し、検査器は観測の区間がその区間を含むことを確かめます。このため、包含区間の観測が `verified` と判定されれば、観測の区間も真の座標を含みます。

- 定理が保証する数値の範囲: [観測形式の説明](protocol/README.md#検証の数値範囲)

これらの定理の証明は、`lake build` で検査器を構築する時に Lean が検査するので、構築が成功すれば、証明が通ることを手元で確かめられます。

## 必要なもの

- Lean: `lean/lean-toolchain` が指す版
- Rust: stable のツールチェーン
- Git: 本リポジトリの複製と、Lake が取得する Git 依存の取得に使う
- `mathlib`: Lake が取得する

## 複製と構築設定

本リポジトリを `git clone` で複製し、タグ `v0.2.5` をチェックアウトします。複製先のディレクトリ名は任意です。以降のコマンドは、複製のルートをカレントディレクトリとした相対パスで示します。

```sh
git clone https://github.com/barineco/neco-geometry.git
cd neco-geometry
git checkout v0.2.5
```

検査器と生成器の構築設定は、次の二つです。

- [`lean/lakefile.toml`](lean/lakefile.toml): 検査器の構築設定
- [`rust/crates/geometry-observation-producer/Cargo.toml`](rust/crates/geometry-observation-producer/Cargo.toml): 生成器の構築設定

検査器の構築設定は、依存する Lean ライブラリを Lake の Git 依存として宣言しています。

- `neco-lean`: [公開版 0.1.0 のコミット](https://github.com/barineco/neco-lean/commit/bf70b4cb63a6bfb18c693e601fea7cb904a455c8)
- `neco-algebra-verify`: [公開版 0.2.0 のコミット](https://github.com/barineco/neco-algebra/commit/1fbbc3d3afdd936473fca88f360d6b8f03517477)
- `neco-calculus-verify`: [公開版 0.2.0 のコミット](https://github.com/barineco/neco-calculus/commit/098885e7b2a0940b7541a20a28f099dc778eafc8)

ルートのこの宣言は、`neco-calculus-verify` と `neco-algebra-verify` が自身の構築設定に持つ相対パスの依存宣言より優先する設定です。

生成器の構築設定は、公開レジストリの依存を Cargo で取得します。幾何ライブラリクレートだけは、同じ複製の中にあるため、相対パスで参照します。

## Lean の検査器

複製のルートで、`verification/lean` ディレクトリに移動し、宣言した Git 依存を取得し、mathlib の構築済みの成果物を取得してから検査器を構築します。

```sh
cd verification/lean
lake update
lake exe cache get
lake build
```

構築した検査器は、観測バッチを読み取り、結果を標準出力へ書き出します。

```sh
.lake/build/bin/neco-geometry-checker check <観測バッチ>
```

## Rust の生成器

複製のルートで、`verification/rust` ディレクトリに移動し、生成器を構築し、実行します。

```sh
cd verification/rust
cargo build --locked
cargo run --locked -p geometry-observation-producer -- describe
cargo run --locked -p geometry-observation-producer -- generate --request <request>
cargo run --locked -p geometry-observation-producer -- self-check --request <request> --batch <観測バッチ>
```

- `describe`: 生成器の版、対象の分野、構築に使った Rust の版などを出力する
- `generate`: 生成要求に対する観測バッチを出力する
- `self-check`: 生成要求から公開 API で観測を作り直し、指定した観測バッチとバイト列が一致するかを確かめる

`<request>` には、生成要求のファイルを指定します。生成要求は正規の JSON で書き、その規約は [観測形式の共通規約](protocol/README.md#共通規約) にあります。`-` を指定すると、生成要求を標準入力から読み取ります。

## 観測器

複製のルートで、`verification/observer` ディレクトリに移動し、保存した観測バッチと検査器の結果を照合し、判定の件数を出力します。

```sh
cd verification/observer
cargo run --locked -- --batch <観測バッチ> --checker-result <検査器の結果>
```

観測器は、生成要求の集合 ( 以下、コーパス ) も作ります。コーパスの元は、対象の分野、演算族、乱数の種、実行する検証項目 ( 後述 ) を宣言した JSON で、これを論理マニフェスト ( logical manifest ) と呼びます。論理マニフェストから作ったコーパスは、`--shard-size` 件ずつのシャード ( shard ) に分けて `--out` のディレクトリへ書き出します。

```sh
cargo run --locked -- generate \
  --logical-manifest manifests/smoke.json \
  --host-execution-config host-execution/host.json \
  --shard-size <n> \
  --out <dir>
```

コーパスの各生成要求は、論理マニフェストの宣言と、コーパスの中の通し番号から一意に決まります。要求に含まれる乱数の値も、論理マニフェストの乱数の種と通し番号から計算します。生成器は、演算族ごとに観測を分ける区分 ( bucket ) を宣言しており、既定のコーパスの件数は、全ての区分に 1 件以上の観測が入る最小の件数です。

大きな件数を使う場合は、論理マニフェストの `generator` に、全体の件数と分野ごとの件数を明示します。

- `normalCaseCount`: 全体の件数
- `allocation`: 分野ごとの件数

件数を明示した論理マニフェストの例は、`manifests/standard.json` ( 50,000 件 ) と `manifests/stress.json` ( 100,000 件 ) です。

## 7 種類の検証項目

観測器の `run-lanes` が実行する検証の単位を、以下、検証項目 ( lane ) と呼びます。代数、解析、幾何の三分野の検証項目は、幾何の観測器が一度にまとめて実行します。実行する検証項目は、`--lanes` に名前をコンマで区切って渡すことで選びます。

- `observationVerification`: 検査器の判定が、観測器による照合の結果と一致すること
- `soundnessTheorem`: 各分野の検査器の `soundness` コマンドを実行し、その出力を記録すること
- `publicApiParity`: 公開 API を再実行した結果が、観測と一致すること
- `mutationRejection`: 改変した観測を検査器が棄却すること
- `invalidProtocol`: 不正な入力を拒否すること
- `corpusExecution`: 宣言したコーパスを生成し、実行できること
- `repeatability`: 2 回の順次実行の結果が一致すること

改変した観測の棄却と不正な入力の拒否を確かめる二つの検証項目は、論理マニフェストが選ぶ全ての分野で、その分野の観測と検査器を使って実行し、結果を分野ごとに分けて報告に記録します。また、観測器が検査器の結果を受理する条件は、検査器へ渡した観測バッチのバイト列の SHA-256 が、結果の `digest` と一致することです。

代数と解析の検査器と生成器は、この文書の対象外です。利用者は、各分野の観測形式と、生成器・検査器の CLI の規約に従う実行ファイルを用意します。この文書が構築手順を示すのは、幾何の検査器と生成器、幾何の観測器だけです。これらは、上の「Lean の検査器」「Rust の生成器」「観測器」の手順で構築したものを使います。

```sh
(cd verification/rust && cargo build --release --locked)
(cd verification/observer && cargo build --release --locked)
```

作業ディレクトリ `<work>` には、生成器と検査器を分野ごとに分けて置きます。ホスト設定 ( `--host-execution-config` に渡す JSON ファイル ) は、これらのパスを参照します。

- `staged/neco.<分野>/producer`: 生成器をリリースビルドで構築した実行ファイル
- `staged/neco.<分野>/checker`: 検査器の実行ファイル `neco-<分野>-checker`

検査器の実行ファイルの名前は `neco-<分野>-checker` の形にそろっていますが、生成器の実行ファイルの名前は、分野ごとに異なる形です。

- algebra: `neco-algebra-producer`
- calculus: `calculus-observation-producer`
- geometry: `geometry-observation-producer`

ホスト設定は、項目名を UTF-8 のバイト順に並べ、余白を省いた正規の JSON で書きます。作業ディレクトリのパスだけが絶対パスで、ほかのパスは作業ディレクトリからの相対パスです。ホスト設定の例は、三分野の生成器と検査器を上の配置で登録したものです。

```json
{"domains":{"neco.algebra":{"checkerPath":"staged/neco.algebra/checker","producerPath":"staged/neco.algebra/producer"},"neco.calculus":{"checkerPath":"staged/neco.calculus/checker","producerPath":"staged/neco.calculus/producer"},"neco.geometry":{"checkerPath":"staged/neco.geometry/checker","producerPath":"staged/neco.geometry/producer"}},"reportDirectory":"reports","temporaryDirectory":"temporary","workingDirectory":"<work>"}
```

コーパス全体を 1 回実行する間に並列で処理するシャードの数は、ホスト設定の `shardParallelism` に 1 以上の整数で指定します。
この項目は省略でき、省略時は実行環境が報告する並列度を使います。

結果はシャードの順序で集約するため、並列度が違っても、報告のバイト列は同じです。

1 つ目のコマンドは検証項目を実行し、2 つ目のコマンドはコーパス全体の実行を 2 回順に行って再現性を確かめます。`H` は、ホスト設定のファイルのパスに置き換える箇所です。

```sh
neco-geometry-observer run-lanes --logical-manifest manifests/smoke.json \
  --host-execution-config H \
  --lanes observationVerification,soundnessTheorem,publicApiParity,mutationRejection,invalidProtocol,corpusExecution,repeatability \
  --require-shared-normal-corpus --require-structured-process-failures
neco-geometry-observer compare-repeatability --logical-manifest manifests/standard.json \
  --host-execution-config H --runs 2 --sequential --shard-size 2000 --write-report <report>
```

1 つ目のコマンドの報告は、作業ディレクトリの `reports/lanes.json` に保存されます。2 つ目のコマンドには、`manifests/stress.json` の論理マニフェストも同じ形で渡すことができます。
