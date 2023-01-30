# なにこれ
面倒くさい作業を自動化しよう。

1. 複数行をコピーしよう
2. ペーストしてみよう
3. みんな嬉しい

[使い方はこっち（動画説明あり）](https://qiita.com/segfo/items/7c92c9401dd1ce5ad02f)

# インストールの方法
[リリース画面](https://github.com/segfo/multiline_paster/releases)から好きなものを持ってってくださいな。

|置いてあるもの|内容|おすすめ度合い|
|---|---|---|
|full_package|モディファイア（プラグイン）と実行ファイルのセット|万人向け|
|plugins.zip|モディファイアとモディファイアインストール用のスクリプト|cargo install --gitでインストールする人におすすめ|

あとは自由にパスを通してよしなに使ってください。

# 機能概要
|機能|v1.6.x|v2.0.0+|
|---|---|---|
|キーボードエミュレーション|O|O
|クリップボード経由のコピー|O|O
|拡張機能|X|O|
|ホットキー無効化|X|O|
|起動中のペーストモード切り替え|X|O|
|コピーのundo機能|X|O|

モディファイアのインストール先は、インストールされたバイナリと同じディレクトリに作成される`multiline_paster_plugins`フォルダです。  
実行したディレクトリに有るconfig.tomlに記述することで変更できます。  
また、`logic_config.toml`はモディファイアの読み込みなど、プログラムの挙動を細かく変更できます。

## 設定ファイルの説明
|キー|役割|
|---|---|
|tabindex_key|TabIndexの通り、ペーストが完了した際タブを押下して次のフォームへ移動しようと試みます。(Clipboardモードでは無視されます)|
|line_delay_msec|1行ペーストした後に入るディレイ（ミリ秒）を設定します。連続で複数行ペーストする際の挙動を安定させる役割があります。（バースト入力モードでのみ使用されます）|
|char_delay_msec|1文字入力毎に入るディレイ（ミリ秒）を設定します。特に意味はありません。ライブコーディングを成功させたいような時に使える機能です。ブラッディ・マンデイみのあるデモ動画に使えるかもしれません。|
|max_line_length|ペースト時にクリップボードモードに切り替える1行の最低文字列長（閾値）です。1行でこの文字数を超えるテキストをペーストしようとした場合にはクリップボードモードでペーストされます。0で解除できます。|
|paste_timeout|（クリップボードモード時のみ有効な設定です）ペーストに時間がかかる場合はCTRL+Vのキーシーケンスがペースト対象に配信されず、ペースト操作がキャンセルされる場合があります。その際に調整します。おおよそ0\[ms\]～300\[ms\]の間で設定すれば良いはずです。<br>一言で言うと：クリップボードモードでペーストできない時は「ペーストにかかった時間」がコンソールに表示されます。その時間よりも小さい値を設定してみてください。|
|text_modifiers|モディファイアです。ペースト時に文字列をエンコードするように動作します。様々な機能があります。詳しくは各モディファイアのヘルプ（`multiline_paster --installed-modifiers`）を確認してください。|

### 設定ファイルのサンプル
```logic_config.toml
tabindex_key = "\t"
line_delay_msec = 200
char_delay_msec = 0
paste_timeout = 100
max_line_length = 256
# 以下のようにコメントアウトも出来ます
# text_modifiers=["multiline_paster_encoder_jwt.dll"]
text_modifiers=["multiline_paster_encoder_jwt.dll","multiline_paster_encoder_rot13.dll"]
```

# ツールの使い方
## バースト入力モード時（--burstオプション有効時）
|ショートカット|動作|
|---|---|
|CTRL+V|自動連続ペースト|

## 非バースト入力モード時
|ショートカット|動作|
|---|---|
|CTRL+V|手動連続ペースト|

## 共通
|ショートカット|動作|
|---|---|
|CTRL+C|連続コピー|
|CTRL+ALT+C|クリップボード内データを全削除する|
|CTRL+ALT+Z|クリップボード内データに対してアンドゥします|
|CTRL+ALT+1|1番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+2|2番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+3|3番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+4|4番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+5|5番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+6|6番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+7|7番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+8|8番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+9|9番目に読み込んだモディファイアの有効化・無効化を切り替えます|
|CTRL+ALT+Q|モディファイアパレットをひとつ先に進めて、切り替えます|
|CTRL+ALT+SHIFT+Q|モディファイアパレットをひとつ前に戻して、切り替えます|
|CTRL+ALT+M|バーストモード・通常入力モードを切り替えます|
|CTRL+ALT+SHIFT+M|クリップボード入力モードと、キーボードエミュレーションモードを切り替えます|
|CTRL+ALT+0|アプリの機能を一時的に停止します。もう一度押すとアプリの機能を使えるようになります|

### モディファイアパレットって何？
`CTRL+ALT+<番号キー>`でモディファイアの有効化・無効化出来ます。  
ですが、単にそれだけだと9個までしか有効化・無効化出来ません。  
なのでユーザが自由に拡張できるようにしました。  
  
モディファイアを10個でも20個追加して、全部読み込んだとしても、パレットを進めることで  
最初は1-9個目までのモディファイアしか`CTRL+ALT+<番号キー>`で有効化・無効化出来ませんが  
`CTRL+ALT+Q`を押すことで、10-18個目のモディファイアも1～9キーで有効化・無効化出来ます。  
もう一度`CTRL+ALT+Q`を押すことで、更に19-26個目のモディファイアを1～9キーで有効化・無効化出来ます。  
  
でも、そんなにたくさんのモディファイアで何するの？？って思うのは作者だけでしょうか。  

## モディファイアの合成
モディファイアを同時に読み込むと、読み込んだ順に適用していきます。  

1. JWTをデコードするモディファイア
2. Base64エンコードするモディファイア
3. URLエンコードをするモディファイア

これらを上記の順序で読み込んだ場合には1～3の順番でエンコードがかかっていきます  
1. 元の文字列がJWTの場合はデコードされる（そうでない場合はそのまま）  
2. 次にBase64エンコードされる  
3. 更にそのあとURLエンコードを行う  
これらの処理が行われた文字列がペーストされる。  

といったような感じです。
合成順序は今のところ逆順には出来ません。  
（やろうと思えば出来ますが、一旦は満足です。もしやる気の有る方が居たらプルリク送ってください。）

# 動作モードについて
動作モードは2種類あります。
1. キーボードエミュレーションモード
2. Clipboard経由モード

それぞれの特徴を説明します。
## キーボードエミュレーションモード
キーボード入力をエミュレーションするモードです。  
デフォルトの挙動になります。  
### メリット
- クリップボードのデータを上書きしません
- コピペ禁止フォームに入力できます
- バースト入力モードが使えます

### デメリット
- 入力がClipboard経由モードよりも若干ゆっくりです

### 使い方
クリップボードモードで起動していても`CTRL+ALT+SHIFT+M`で相互切り替えが出来ます。
ペースト対象のアプリのIMEを切って使ってください。  
とにかくIMEを切れ。  
いいな、IMEを切るんだ。  
将来的にはIMEを自動的にオフる機能をつけるつもり。めんどいし。  

## Clipboard経由モード
クリップボードに存在するテキストデータを1行ごと上書きするモードです。  
### メリット
- IMEのモードに関わらず常にコピーしたデータが寸分違わずにペーストされます
- 長大なテキストに関してのペースト速度は、キーボードエミュレーションモードより高速です。
### デメリット
- クリップボードのデータが上書きされていきます。
- バーストモードは使えません。

### 使い方
`--clipboard`オプションを有効化することで使えます。
例：`multiline_paster --clipboard`
オプション無しで起動しても、`CTRL+ALT+SHIFT+M`でキーボードエミュレーションモードとの相互切り替えが出来ます。
また、長大な文字列（`logic_config.toml`のmax_line_lengthで定義される文字数以上）の場合クリップボードモードでペーストされます。

## バースト入力モード（通称：バーストモード）
TABキーを自動で入力して隣のフォームに移動しながら入力するモードです。  
`multiline_paster --burst`
v1.4～v1.6.xのバージョンでは、`config.toml`を変更することでフォーム移動がTABキー以外の場合でも対応できます。（矢印キーは除く）  
v2.0以降のバージョンでは`logic_config.toml`を変更することで同様に変更できます。
クリップボードモードでは使えません。  
（もしクリップボードモードで使いたかったらいい感じに改造してPull Request送ってほしい）  

## 連続コピー機能（v1.6+）
ペーストが連続で出来るのに、なんでコピーを連続で出来ないんだ？  
エクスペリエンス的にクソでは？と思ったので追加しました。

1. 以下のテキストに従って、1行ずつ`CTRL+C`を押します。
```
1行目：この行を1番目にコピーしてね
2行目：この行を3番目にコピーしてね
3行目：この行を4番めにコピーしてね
4行目：この行を2番めにコピーしてね
```
2. ペーストします。
```
1行目：この行を1番目にコピーしてね
4行目：この行を2番めにコピーしてね
2行目：この行を3番目にコピーしてね
3行目：この行を4番めにコピーしてね
```
コピーした順にペーストされるよ！  
という機能でございます。