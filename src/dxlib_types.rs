/*dxlib struct types*/
use std::ops::*;
use std::os::raw::*;
#[repr(C)]
pub struct RECT {
    pub left: CInt,
    pub top: CInt,
    pub right: CInt,
    pub bottom: CInt,
}
#[repr(C)]
pub struct VECTOR {
    pub x: CFloat,
    pub y: CFloat,
    pub z: CFloat,
}
impl Add for VECTOR {
    type Output = VECTOR;
    fn add(self, other: VECTOR) -> VECTOR {
        return VECTOR {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        };
    }
}
impl Clone for VECTOR {
    fn clone(&self) -> Self {
        VECTOR {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}
#[repr(C)]
pub struct COLOR_U8 {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
#[repr(C)]
pub struct COLOR_F {
    pub a: CFloat,
    pub r: CFloat,
    pub g: CFloat,
    pub b: CFloat,
}

#[repr(C)]
pub struct MATERIALPARAM {
    // ディフューズカラー
    pub Diffuse: COLOR_F,
    // アンビエントカラー
    pub Ambient: COLOR_F,
    // スペキュラカラー
    pub Specular: COLOR_F,
    // エミッシブカラー
    pub Emissive: COLOR_F,
    // スペキュラの強さ
    pub Power: CFloat,
}
#[repr(C)]
// ３Ｄ描画に使用する頂点データ型
pub struct VERTEX3D {
    // 座標
    pub pos: VECTOR,

    // 法線
    pub norm: VECTOR,

    // ディフューズカラー
    pub dif: COLOR_U8,

    // スペキュラカラー
    pub spc: COLOR_U8,

    // テクスチャ座標
    pub u: CFloat,
    pub v: CFloat,

    // サブテクスチャ座標
    pub su: CFloat,
    pub sv: CFloat,
}
#[repr(C)]
pub struct HITRESULT_LINE {
    // 接触しているかどうか( 1:接触している  0:接触していない )
    pub HitFlag: CInt,
    // 接触した座標( HitFlag が 1 の場合のみ有効 )
    pub Position: VECTOR,
}
#[repr(C)]
pub struct DATEDATA {
    pub Year: CInt, // 年
    pub Mon: CInt,  // 月
    pub Day: CInt,  // 日
    pub Hour: CInt, // 時間
    pub Min: CInt,  // 分
    pub Sec: CInt,  // 秒
}
// DirectX
#[repr(C)]
pub struct XAUDIO2FX_REVERB_PARAMETERS {
    pub WetDryMix: CFloat,
    pub ReflectionsDelay: u32,
    pub ReverbDelay: u8,
    pub RearDelay: u8,
    pub PositionLeft: u8,
    pub PositionRight: u8,
    pub PositionMatrixLeft: u8,
    pub PositionMatrixRight: u8,
    pub EarlyDiffusion: u8,
    pub LateDiffusion: u8,
    pub LowEQGain: u8,
    pub LowEQCutoff: u8,
    pub HighEQGain: u8,
    pub HighEQCutoff: u8,
    pub RoomFilterFreq: CFloat,
    pub RoomFilterMain: CFloat,
    pub RoomFilterHF: CFloat,
    pub ReflectionsGain: CFloat,
    pub ReverbGain: CFloat,
    pub DecayTime: CFloat,
    pub Density: CFloat,
    pub RoomSize: CFloat,
}

#[repr(C)]
pub struct IPDATA {
    d1: c_uchar,
    d2: c_uchar,
    d3: c_uchar,
    d4: c_uchar,
}
#[repr(C)]
pub struct MATRIX {
    pub m: [[CFloat; 4]; 4],
}
impl Clone for MATRIX {
    fn clone(&self) -> Self {
        MATRIX { m: self.m }
    }
}
//pub type TCHAR = u16;

pub type CFloat = f32;
pub type CInt = i32;
pub type CVoid = c_void;
pub type CLongLong = i64;
pub type CDouble = f64;
pub type CChar = c_char;

