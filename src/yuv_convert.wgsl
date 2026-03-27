// yuv_convert.wgsl

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> output_yuv: array<u32>;

struct Config {
    width: u32,
    height: u32,
};

// 這是標準的 BT.601 常數 (RGBA -> YUV)
const R_TO_Y = 0.299;
const G_TO_Y = 0.587;
const B_TO_Y = 0.114;

const R_TO_U = -0.169;
const G_TO_U = -0.331;
const B_TO_U = 0.500;

const R_TO_V = 0.500;
const G_TO_V = -0.419;
const B_TO_V = -0.081;

// 輔助函數：讀取特定座標並轉為 Y
fn get_y(pos: vec2<u32>) -> u32 {
    let rgb = textureLoad(input_tex, pos, 0).rgb;
    let y = R_TO_Y * rgb.r + G_TO_Y * rgb.g + B_TO_Y * rgb.b;
    return u32(clamp(y * 255.0, 0.0, 255.0));
}

// 輔助函數：讀取 2x2 區域並平均轉為 U 或 V
fn get_uv(pos_tl: vec2<u32>, mode: u32) -> u32 {
    // mode 0 = U, 1 = V
    var res = 0.0;
    // 採樣 2x2 像素求平均（YUV420 降採樣）
    for(var i=0u; i<2u; i++) {
        for(var j=0u; j<2u; j++) {
            let rgb = textureLoad(input_tex, pos_tl + vec2(i, j), 0).rgb;
            if (mode == 0u) {
                res += (R_TO_U * rgb.r + G_TO_U * rgb.g + B_TO_U * rgb.b + 0.5);
            } else {
                res += (R_TO_V * rgb.r + G_TO_V * rgb.g + B_TO_V * rgb.b + 0.5);
            }
        }
    }
    return u32(clamp((res / 4.0) * 255.0, 0.0, 255.0));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let width = textureDimensions(input_tex).x;
    let height = textureDimensions(input_tex).y;
    
    // 為了讓一個線程寫入一個 u32 (4 bytes)，我們讓 id.x 代表輸出的「字組」位置
    // 假設 id.x 在 [0, width/4), id.y 在 [0, height * 1.5)
    
    let out_x = id.x;
    let out_y = id.y;
    
    if (out_x * 4 >= width || out_y >= height + height / 2) { return; }

    var packed_val = 0u;

    if (out_y < height) {
        // --- Y 區域填寫 ---
        let src_x = out_x * 4;
        let src_y = out_y;
        
        let y0 = get_y(vec2(src_x + 0, src_y));
        let y1 = get_y(vec2(src_x + 1, src_y));
        let y2 = get_y(vec2(src_x + 2, src_y));
        let y3 = get_y(vec2(src_x + 3, src_y));
        
        // 依照 Little Endian 打包
        packed_val = y0 | (y1 << 8) | (y2 << 16) | (y3 << 24);
        
    } else if (out_y < height + height / 4) {
        // --- U 區域 ---
        let uv_row = out_y - height;
        let src_x = out_x * 8; // 因為 U 是寬度減半，一個輸出 u32 對應原圖 8 個橫向像素
        let src_y = uv_row * 2;
        
        let u0 = get_uv(vec2(src_x + 0, src_y), 0u);
        let u1 = get_uv(vec2(src_x + 2, src_y), 0u);
        let u2 = get_uv(vec2(src_x + 4, src_y), 0u);
        let u3 = get_uv(vec2(src_x + 6, src_y), 0u);
        
        packed_val = u0 | (u1 << 8) | (u2 << 16) | (u3 << 24);
        
    } else {
        // --- V 區域 ---
        let uv_row = out_y - (height + height / 4);
        let src_x = out_x * 8;
        let src_y = uv_row * 2;
        
        let v0 = get_uv(vec2(src_x + 0, src_y), 1u);
        let v1 = get_uv(vec2(src_x + 2, src_y), 1u);
        let v2 = get_uv(vec2(src_x + 4, src_y), 1u);
        let v3 = get_uv(vec2(src_x + 6, src_y), 1u);
        
        packed_val = v0 | (v1 << 8) | (v2 << 16) | (v3 << 24);
    }

    let dest_idx = out_y * (width / 4) + out_x;
    output_yuv[dest_idx] = packed_val;
}