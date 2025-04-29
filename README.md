# DxLibをRust用で扱うためのRust向けラッパー
## dxlib-rs使い方
### 手順
1. DxLibの公式サイトからC#用をダウンロードし、解凍後、DxLib_x64.dllかDxLib.dll等をプロジェクトルートに配置
2. dxlib-ffiを追加 
```plaintext
    cargo add --git https://github.com/mAru0090/dxlib-rs
```
3. anyhowを追加
```plaintext
    cargo add anyhow
```
4. 下記サンプルをコピペ等して実行
```rust
use dxlib_rs::dxlib::*;
use dxlib_rs::dxlib_constants::*;
use dxlib_rs::dxlib_types::*;
use anyhow::Result;
fn main() ->Result<()> {
    SetUseCharCodeFormat(DX_CHARCODEFORMAT_UTF8)?;
    SetMainWindowText("DxLib and Rust draw Window! DxLibとRustでウィンドウ表示!")?;
    ChangeWindowMode(TRUE)?;
    DxLib_Init()?;
    SetDrawScreen(DX_SCREEN_BACK)?;
    let mut rect = RECT {
        left: -1,
        right: -1,
        top: -1,
        bottom: -1,
    };
    while ScreenFlip().is_ok() && ClearDrawScreen(&mut rect).is_ok() && ProcessMessage().is_ok() {
        DrawString(0, 0, "hello world! こんにちは 世界!", GetColor(255, 255, 255)?)?;
    }
    DxLib_End()?;
    Ok(())
}
```
5. もしdxlib_rs::dxlib内で定義されているdxlib関数が足りない場合は、下記の様に自身で定義
```rust
// =======================================================
// dxlib-rs版
// dxlib_gen! {
//  [libname],
//  [signature*]
// }
// CInt,CChar等その他C用のtype宣言、構造体はdxlib_typesに記述されている
// DxLib特有の定数は、dxlib_constantsに記述されている
// 基本的にDxLibの関数シグネチャ通りに渡すことが可能
// 生成される関数の戻り値は、すべてanyhow::Result<指定した型,DxLibError>に変換される
// Option型を引数に使用した場合、引数前に#[default = "0"]等することで、None時に渡す値を設定可能
// Resultのエラー分岐条件を変更したい場合、関数宣言前に、[#error_confition = "result == 0"]などをすることが可能
// &strを指定した場合のみ、*const i8として変換されて渡される
// =======================================================
dxlib_gen! {
    // ライブラリ名
    "DxLib_x64",
    // ライブラリの初期化
    fn DxLib_Init() -> CInt,
    // ライブラリ使用の終了関数
    fn DxLib_End() -> CInt,
    // ウインドウズのメッセージを処理する
    fn ProcessMessage() -> CInt,
    // フリップ関数、画面の裏ページ(普段は表示されていない)の内容を表ページ(普段表示されている)に反映する
    fn ScreenFlip() -> CInt,
    // 描画先グラフィック領域の指定
    fn SetDrawScreen(#[default = "DX_SCREEN_BACK"] draw_screen: Option<CInt>) -> CInt,
    fn ClearDrawScreen(clear_rect: *mut RECT) -> CInt,
    // ウインドウモード・フルスクリーンモードの変更を行う
    fn ChangeWindowMode(#[default = "1"] flag:Option<CInt>) -> CInt,
    // ウインドウのタイトルを変更する
    fn SetMainWindowText(window_text: &str) -> CInt,
    // キーの入力待ち
    #[error_condition = "result == i32::MAX"]
    fn WaitKey() -> CInt,
    // キーボードによる文字列の入力
    fn KeyInputString(
        x: CInt,
        y: CInt,
        char_max_length: CInt,
        str_buffer: *mut CChar,
        cancel_valid_flag: CInt,
    ) -> CInt,
    // 文字列の引数の文字コードを設定する
    fn SetUseCharCodeFormat(
        char_code_format: CInt,
    ) -> CInt,
    // 色コードを取得する
    #[error_condition = "result == i32::MAX"]
    fn GetColor(red: CInt, green: CInt, blue: CInt) -> CInt,
    // 文字列を描画する
    fn DrawString(x: CInt, y: CInt, string: &str, color: CInt) -> CInt,
}
```
6. cargo run等して実行
