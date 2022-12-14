# なにこれ
面倒くさい作業を自動化しよう。

1. 複数行をコピーしよう
2. ペーストしてみよう
3. みんな嬉しい

[使い方はこっち（動画説明あり）](https://qiita.com/segfo/items/7c92c9401dd1ce5ad02f)

# インストールの方法
## Rustがインストールされている場合
推奨
```
cargo install --git=https://github.com/segfo/multiline_paster
```

自分でバイナリを配置したい場合
```
git clone --recursive https://github.com/segfo/multiline_paster/
cd multiline_paster
cargo build --release
```

## Rustがインストールされてない場合
[リリース画面](https://github.com/segfo/multiline_paster/releases)から好きなものを持っていくが良い。

# 概要
```
multiline_paster.exe -h    
Usage: multiline_paster.exe [OPTIONS]

Options:
      --clipboard  動作モードがクリップボード経由でペーストされます（デフォルト：キーボードエミュレーションでのペースト） 本モードはバーストモードと排他です。
      --burst      バーストモード（フォームに対する連続入力モード）にするか選択できます。
  -h, --help       Print help information
  -V, --version    Print version information
```

# 動作モードについて
動作モードは2種類あります。
1. DirectInputモード
2. Clipboard経由モード

それぞれの特徴を説明します。
## DirectInputモード
キーボード入力をエミュレーションするモードです。  
デフォルトの挙動になります。  
### メリット
- クリップボードのデータを上書きしません
- コピペ禁止フォームに入力できます
- バースト入力モードが使えます

### デメリット
- 入力がClipboard経由モードよりも若干ゆっくりです

### 使い方
ペースト対象のアプリのIMEを切って使ってください。  
とにかくIMEを切れ。  
いいな、IMEを切るんだ。  
将来的にはIMEを自動的にオフる機能をつけるつもり。めんどいし。  

## Clipboard経由モード
クリップボードに存在するテキストデータを1行ごと上書きするモードです。  
### メリット
- IMEのモードに関わらず常にコピーしたデータが寸分違わずにペーストされます
- DirectInputよりも高速です
### デメリット
- クリップボードのデータが上書きされていきます。
- バーストモードは使えません。

### 使い方
`--clipboard`引数を付与することで有効になります。

（例）
`multiline_paster --clipboard`

## バースト入力モード（通称：バーストモード）
TABキーを自動で入力して隣のフォームに移動しながら入力するモードです。  
`multiline_paster --burst`
v1.4以降のバージョンで全機能に対応しています。  
クリップボードモードでは使えません。  
（もしクリップボードモードで使いたかったらいい感じに改造してPull Request送ってほしい）  
