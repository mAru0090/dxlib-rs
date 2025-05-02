#![allow(non_snake_case)]

use crate::dxlib_constants::*;
use crate::dxlib_error::*;
use crate::dxlib_types::*;
use crate::utils::*;

use dxlib_rs_macro::dxlib_gen;

static mut DEFAULT_RECT: RECT = RECT {
    left: -1,
    right: -1,
    top: -1,
    bottom: -1,
};
fn default_rect_ptr() -> *mut RECT {
    unsafe { &raw mut DEFAULT_RECT }
}
// =======================================================
// dxlib-rs版
// dxlib_gen! {
//  [libname],
//  [signature*]
// }
// DxLib特有の定数は、dxlib_constantsに記述されている
// 基本的にDxLibの関数シグネチャ通りに渡すことが可能
// 関数シグネチャの前に#[alias = "dxlib_init"]等をした場合、エイリアスをつけて関数生成が可能
// 関数シグネチャの前に#[not_result]等をした場合、その関数はanyhow::Result<指定した型,DxLibError>に変換されなくなる
// 生成される関数の戻り値は、すべてanyhow::Result<指定した型,DxLibError>に変換される
// Option型を引数に使用した場合、引数前に#[default = "0"]等することで、None時に渡す値を設定可能
// Resultのエラー分岐条件を変更したい場合、関数宣言前に、[#error_confition = "result == 0"]などをすることが可能
// &str,String,&Stringを指定した場合のみ、*const i8として変換されて渡される
// Into<Vec<u8>>,Into<Vec<i8>>,Into<Vec<T>>,Vec<T>,&Into<Vec<u8>>,&Into<Vec<i8>>,&Into<Vec<T>>,&Vec<T>の場合のみ、*const Tに変換されて渡される
// &mut Vec<T>の場合のみ、*mut Tに変換されて渡される
// =======================================================
dxlib_gen! {
    // ライブラリ名
    "DxLib_x64",
    // ライブラリの初期化
    //#[alias = "dxlib_init"]
    //#[not_result]
    fn DxLib_Init() -> i32,
    // ライブラリ使用の終了関数
    fn DxLib_End() -> i32,
    // ウインドウズのメッセージを処理する
    fn ProcessMessage() -> i32,
    // フリップ関数、画面の裏ページ(普段は表示されていない)の内容を表ページ(普段表示されている)に反映する
    fn ScreenFlip() -> i32,
    // 描画先グラフィック領域の指定
    fn SetDrawScreen(#[default = "DX_SCREEN_BACK"] draw_screen: Option<i32>) -> i32,
    fn ClearDrawScreen(#[default = "default_rect_ptr()"] clear_rect: Option<*mut RECT>) -> i32,
    // ウインドウモード・フルスクリーンモードの変更を行う
    fn ChangeWindowMode(#[default = "1"] flag:Option<i32>) -> i32,
    // ウインドウのタイトルを変更する
    fn SetMainWindowText(window_text: impl ToString) -> i32,
    // キーの入力待ち
    #[error_condition = "result == i32::MAX"]
    fn WaitKey() -> i32,
    // キーボードによる文字列の入力
    fn KeyInputString(
        x: i32,
        y: i32,
        char_max_length: i32,
        str_buffer: &mut Vec<std::os::raw::c_char>,
        cancel_valid_flag: i32,
    ) -> i32,
    // 文字列の引数の文字コードを設定する
    fn SetUseCharCodeFormat(
        char_code_format: i32,
    ) -> i32,
    // 色コードを取得する
    #[error_condition = "result == i32::MAX"]
    fn GetColor(red: i32, green: i32, blue: i32) -> i32,
    // 文字列を描画する
    fn DrawString(x: i32, y: i32, string: impl ToString, color: i32) -> i32,
    fn LoadGraph(file_name: impl ToString) -> i32,
    fn DrawGraph(x: i32, y: i32, gr_handle: i32, trans_flag: i32) -> i32,
    fn PlaySoundMem(sound_handle: i32, play_type: i32, top_position_flag: i32) -> i32,
    fn LoadSoundMem(file_name: impl ToString) -> i32,
    #[error_condition = "result == i32::MAX"]
    fn CheckHitKey(key_code: i32) -> i32,
    fn FileRead_open(file_path: impl ToString,r#async: i32) -> i32,
    fn FileRead_size(file_path: impl ToString) -> std::os::raw::c_long,
    fn FileRead_close(file_handle: i32) -> i32,
    fn FileRead_tell(file_handle: i32) -> std::os::raw::c_long,
    fn FileRead_seek(file_handle: i32,offset: std::os::raw::c_long,origin: i32) -> i32,
    fn FileRead_read(buffer: *mut std::os::raw::c_void,read_size: i32,file_handle: i32) -> i32,
    fn FileRead_gets(buffer: &mut Vec<std::os::raw::c_char>,num: i32,file_handle: i32) -> i32,
    fn SetUseASyncLoadFlag(flag: i32) -> i32,
}
