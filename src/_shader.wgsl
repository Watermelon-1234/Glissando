struct VRParams {
    offset: f32,     // 瞳距偏移
    z_distance: f32, // 畫面距離
    k1: f32,         // 畸變係數1
    k2: f32,         // 畸變係數2

    sensitivity: f32,

    current_quat: vec4f, // (x, y, z, w)
    base_quat: vec4f,

    _padding: f32,               
};

// 輔助函數：繞中心點旋轉 UV
fn rotate_uv(uv: vec2f, angle: f32) -> vec2f {
    let s = sin(angle);
    let c = cos(angle);
    let pivot = vec2f(0.5, 0.5); // 畫面中心
    
    // 將原點移至中心
    let temp_uv = uv - pivot;
    
    // 套用旋轉矩陣
    let rotated = vec2f(
        temp_uv.x * c - temp_uv.y * s,
        temp_uv.x * s + temp_uv.y * c
    );
    
    // 移回原處
    return rotated + pivot;
}

fn rotate_vector(v: vec3f, q: vec4f) -> vec3f {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var screen_sampler: sampler;
@group(0) @binding(2)
var<uniform> params: VRParams;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // 1. 標準 NDC 座標 (用於輸出到螢幕位置)
    var pos = array<vec2f, 6>(
        vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
        vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
    );

    let is_right = f32(vertex_index / 6u);
    let local_idx = vertex_index % 6u;
    let p = pos[local_idx];

    // 2. 輸出位置：填滿 SBS 的左右各半邊
    let screen_x = (p.x * 0.5) + (is_right * 1.0 - 0.5);
    // Y 保持不變，直接對應渲染目標的 NDC
    let screen_y = p.y; 

    // 3. 核心修正：UV 採樣座標計算
    let zoom = max(params.z_distance, 0.001);
    let parallax = (is_right - 0.5) * params.offset;
    
    // 修正 X 軸：p.x 是 [-1, 1]，加上視差後轉為 [0, 1]
    var uv_x = (p.x * zoom + parallax + 1.0) / 2.0;
    
    // 修正 Y 軸：關鍵！如果原本上下相反，這裡改為 (1.0 - (p.y * zoom + 1.0) / 2.0)
    // 或者直接將 p.y 加上負號。
    // 這裡使用 1.0 - ... 來翻轉 Y 軸
    var uv_y = 1.0 - ((p.y * zoom + 1.0) / 2.0);

    uv_x = uv_x + (params.yaw * params.sensitivity);
    uv_y = uv_y + (params.pitch * params.sensitivity);

    var uv = vec2f(
        uv_x,
        uv_y,
    );

    uv = rotate_uv(uv, params.roll);

    var out: VertexOutput;
    out.position = vec4f(screen_x, screen_y, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // 1. 將輸入 UV 轉回 [-1, 1] 以計算中心畸變
    let st = in.uv * 2.0 - 1.0;
    let r2 = st.x * st.x + st.y * st.y;

    // 2. 桶狀畸變計算
    let distortion = 1.0 + params.k1 * r2 + params.k2 * r2 * r2;
    let distorted_st = st * distortion;

    // 3. 轉回 [0, 1] 進行最終採樣
    let final_uv = (distorted_st + 1.0) / 2.0;

    // 4. 邊界檢查
    if (final_uv.x < 0.0 || final_uv.x > 1.0 || final_uv.y < 0.0 || final_uv.y > 1.0) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    return textureSample(screen_texture, screen_sampler, final_uv);
}