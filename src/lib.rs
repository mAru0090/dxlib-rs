pub mod dxlib;
pub mod dxlib_constants;
pub mod dxlib_error;
pub mod dxlib_types;
pub mod utils;
pub use dxlib_rs_macro::dxlib_gen;

mod tests {
    use crate::dxlib::*;
    use crate::dxlib_constants::*;
    use crate::dxlib_error::*;
    use crate::dxlib_types::*;
    use anyhow::Result as R;
    use std::f64::consts::PI;
    use std::ffi::CStr;
    use std::os::raw::c_char;
    #[test]
    fn test_dxlib_1() -> R<(), DxLibError> {
        SetUseCharCodeFormat(DX_CHARCODEFORMAT_UTF8)?;
        let window_title = "aiueo!! あいうえお!";
        SetMainWindowText(window_title);
        ChangeWindowMode(None)?;
        DxLib_Init()?;
        SetDrawScreen(None)?;
        SetUseASyncLoadFlag(TRUE)?;

        let center_x = 320.0;
        let center_y = 240.0;
        let radius = 100.0;
        let mut deg = 0;
        let file_buffer_size: usize = 256;

        let mut file_buffer = vec![1i8; file_buffer_size];
        let file_handle = FileRead_open("./test.txt", TRUE)?;
        {
            FileRead_gets(&mut file_buffer, file_buffer_size as i32, file_handle);
            let file_buffer_u8 = file_buffer.iter().map(|&b| b as u8).collect();
            println!("{}", String::from_utf8(file_buffer_u8).unwrap());
        }
        {
            FileRead_gets(&mut file_buffer, file_buffer_size as i32, file_handle);
            let file_buffer_u8 = file_buffer.iter().map(|&b| b as u8).collect();
            println!("{}", String::from_utf8(file_buffer_u8).unwrap());
        }

        FileRead_close(file_handle)?;

        let key_input_size: usize = 1024;
        let key_input_size = 128; // 例: 入力バッファのサイズ
        //let mut key_input: Vec<c_char> = vec![0; key_input_size]; // バッファの初期化
        let mut key_input: [c_char; 1024] = [0; 1024];
        KeyInputString(0, 0, key_input_size as i32, &mut key_input, FALSE); // スライスとして渡す

        // KeyInputString 呼び出し時に Vec<CChar> を &mut [CChar] として渡す
        //KeyInputString(0, 0, key_input_size as i32, &mut key_input[..], FALSE); // スライスとして渡す

        // key_inputからUTF-8の文字列に変換

        let key_input_string = unsafe {
            CStr::from_ptr(key_input.as_ptr()) // ポインタからC文字列を取得
                .to_string_lossy() // UTF-8に変換、無効なバイトはUTF-8のエラーを無視して処理
                .into_owned() // String型に変換
        };
        //let snd =
        //LoadSoundMem("D:/win/program/rb/main-project/youtube-download/touhou-mangetu.mp3")?;
        //PlaySoundMem(snd, DX_PLAYTYPE_LOOP, 0)?;

        while ScreenFlip().is_ok()
            && ClearDrawScreen(None).is_ok()
            && ProcessMessage().is_ok()
            && CheckHitKey(KEY_INPUT_ESCAPE)? == FALSE
        {
            deg = (deg + 1) % 360;
            let angle_rad = deg as f64 * std::f64::consts::PI / 180.0;

            let x = center_x + radius * f64::cos(angle_rad);
            let y = center_y + radius * f64::sin(angle_rad);
            DrawString(
                0,
                0,
                &format!(
                    "x: {:.2} y: {:.2} angle_rad: {:.2} deg: {}",
                    x, y, angle_rad, deg
                ),
                GetColor(255, 255, 255)?,
            )?;
            DrawString(
                x as i32,
                y as i32,
                key_input_string.as_str(),
                GetColor(255, 255, 255)?,
            )?;
        }
        DxLib_End()?;
        Ok(())
    }
}
