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
    #[test]
    fn test_dxlib_1() -> R<(),DxLibError> {
        SetUseCharCodeFormat(DX_CHARCODEFORMAT_UTF8)?;
        ChangeWindowMode(None)?;
        DxLib_Init()?;
        let mut rect = RECT {
            left: -1,
            right: -1,
            top: -1,
            bottom: -1,
        };
        SetDrawScreen(None)?;
        while ScreenFlip().is_ok() && ClearDrawScreen(&mut rect).is_ok() &&ProcessMessage().is_ok() {
            DrawString(0,0,"hello world こんにちは、世界",GetColor(255,255,255)?)?;
        }
        DxLib_End()?;
        Ok(())
    }
}
