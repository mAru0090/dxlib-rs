pub mod dxlib;
pub mod dxlib_constants;
pub mod dxlib_types;
pub mod dxlib_error;
pub use dxlib_rs_macro::dxlib_gen;

mod tests {
    use crate::dxlib::*;
    use crate::dxlib_constants::*;
    use crate::dxlib_types::*;
    use crate::dxlib_error::*;
    use anyhow::Result as R;
    use std::f64::consts::PI;
    #[test]
    fn test_dxlib_1() -> R<(),DxLibError> {
        SetUseCharCodeFormat(DX_CHARCODEFORMAT_UTF8)?;
        ChangeWindowMode(None)?;
        DxLib_Init()?;
        SetDrawScreen(None)?;
        SetUseASyncLoadFlag(TRUE)?;
        let mut x = 320.0;
        let mut y = 240.0;
        let mut vx = 0.0;
        let mut vy = 0.0;
        let mut angle = 0.0;
        let mut deg = 0;
        let v = 2;
        let key_input_size:usize = 256;
        let mut file_buffer = vec![0i8;key_input_size];
        let file_handle = FileRead_open("./build.rs",TRUE)?;
        FileRead_gets(file_buffer.as_mut_ptr(),key_input_size as i32,file_handle);
        let file_buffer_u8 = file_buffer.iter().map(|&b| b as u8).collect();
        println!("{:?}",String::from_utf8(file_buffer_u8));
        FileRead_close(file_handle)?;
        while ScreenFlip().is_ok() && ClearDrawScreen(None).is_ok() &&ProcessMessage().is_ok() && CheckHitKey(KEY_INPUT_ESCAPE)? == FALSE {
            x += vx;
            y += vy;
            deg += 1;
            angle = 0.2 * deg as f64;
            vx = v as f64 * f64::cos(angle*PI / 10.0);
            vy = v as f64 * f64::sin(angle*PI / 10.0);
            DrawString(0,0,&format!("x: {:.2} y: {:.2} vx: {:.2} vy: {:.2} angle: {:.2} deg: {}",x,y,vx,vy,angle,deg),GetColor(255,255,255)?)?;
            DrawString(x as i32,y as i32,"hello world こんにちは、世界",GetColor(255,255,255)?)?;
        }
        DxLib_End()?;
        Ok(())
    }
}
