#![allow(non_snake_case)]

use crate::dxlib_constants::*;
use crate::dxlib_error::*;
use crate::dxlib_types::*;
use crate::utils::*;

use dxlib_rs_macro::dxlib_gen;

const DEFAULT_RECT: RECT = RECT {
    left: -1,
    right: -1,
    top: -1,
    bottom: -1,
};

// =======================================================
// dxlib-rs版
// dxlib_gen! {
//  [libname],
//  [signature*]
// }
// CInt,CChar等その他C用のtype宣言、構造体はdxlib_typesに記述されている
// DxLib特有の定数は、dxlib_constantsに記述されている
// 基本的にDxLibの関数シグネチャ通りに渡すことが可能
// 関数シグネチャの前に#[alias = "dxlib_init"]等をした場合、エイリアスをつけて関数生成が可能
// 関数シグネチャの前に#[not_result]等をした場合、その関数はanyhow::Result<指定した型,DxLibError>に変換されなくなる
// 生成される関数の戻り値は、すべてanyhow::Result<指定した型,DxLibError>に変換される
// Option型を引数に使用した場合、引数前に#[default = "0"]等することで、None時に渡す値を設定可能
// Resultのエラー分岐条件を変更したい場合、関数宣言前に、[#error_confition = "result == 0"]などをすることが可能
// &str,String,&Stringを指定した場合のみ、*const i8として変換されて渡される
// Into<Vec<u8>>,Into<Vec<i8>>,Into<Vec<T>>,Vec<T>の場合のみ、*mut u8,*mut i8 又は、*mut Tに変換されて渡される
// =======================================================
dxlib_gen! {
    // ライブラリ名
    "DxLib_x64",
    // ライブラリの初期化
    //#[alias = "dxlib_init"]
    //#[not_result]
    fn DxLib_Init() -> CInt,
    // ライブラリ使用の終了関数
    fn DxLib_End() -> CInt,
    // ウインドウズのメッセージを処理する
    fn ProcessMessage() -> CInt,
    // フリップ関数、画面の裏ページ(普段は表示されていない)の内容を表ページ(普段表示されている)に反映する
    fn ScreenFlip() -> CInt,
    // 描画先グラフィック領域の指定
    fn SetDrawScreen(#[default = "DX_SCREEN_BACK"] draw_screen: Option<CInt>) -> CInt,
    fn ClearDrawScreen(#[default = "&mut DEFAULT_RECT"] clear_rect: Option<*mut RECT>) -> CInt,
    // ウインドウモード・フルスクリーンモードの変更を行う
    fn ChangeWindowMode(#[default = "1"] flag:Option<CInt>) -> CInt,
    // ウインドウのタイトルを変更する
    fn SetMainWindowText(window_text: impl ToString) -> CInt,
    // キーの入力待ち
    #[error_condition = "result == i32::MAX"]
    fn WaitKey() -> CInt,
    // キーボードによる文字列の入力
    fn KeyInputString(
        x: CInt,
        y: CInt,
        char_max_length: CInt,
        str_buffer: Vec<CChar>,
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
    fn DrawString(x: CInt, y: CInt, string: impl ToString, color: CInt) -> CInt,
    fn LoadGraph(file_name: impl ToString) -> CInt,
    fn DrawGraph(x: CInt, y: CInt, gr_handle: CInt, trans_flag: CInt) -> CInt,
    fn PlaySoundMem(sound_handle: CInt, play_type: CInt, top_position_flag: CInt) -> CInt,
    fn LoadSoundMem(file_name: impl ToString) -> CInt,
    #[error_condition = "result == i32::MAX"]
    fn CheckHitKey(key_code: CInt) -> CInt,
    fn FileRead_open(file_path: impl ToString,r#async: CInt) -> CInt,
    fn FileRead_size(file_path: impl ToString) -> CLongLong,
    fn FileRead_close(file_handle: CInt) -> CInt,
    fn FileRead_tell(file_handle: CInt) -> CLongLong,
    fn FileRead_seek(file_handle: CInt,offset: CLongLong,origin: CInt) -> CInt,
    fn FileRead_read(buffer: *mut CVoid,read_size: CInt,file_handle: CInt) -> CInt,
    fn FileRead_gets(buffer: *mut CChar,num: CInt,file_handle: CInt) -> CInt,
    fn SetUseASyncLoadFlag(flag: CInt) -> CInt,
}
