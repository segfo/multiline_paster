# multiline_paster
面倒くさい作業を自動化しよう。

1. 複数行をコピーしよう
2. ペーストしてみよう
3. みんな嬉しい

# 動作モードについて
動作モードは2種類あります。
1. DirectInputモード
2. Clipboard経由モード

それぞれの特徴を説明します。
## DirectInputモード
キーボード入力をエミュレーションするモードです。  
デフォルトの挙動になります。  
メリット：クリップボードのデータが上書きされません

ペースト対象のアプリのIMEを切って使ってください。  
とにかくIMEを切れ。  
いいな、IMEを切るんだ。  
将来的にはIMEを自動的にオフる機能をつけるつもり。めんどいし。  

## Clipboard経由モード
クリップボードに存在するテキストデータを1行ごと上書きするモードです。  
メリット：IMEのモードに関わらず常にコピーしたデータが寸分違わずにペーストされます  
デメリット：クリップボードのデータが上書きされていきます。  

`--clipboard`引数を付与することで有効になります。

（例）
`multiline_paster --clipboard`
