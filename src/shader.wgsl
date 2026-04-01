struct VRParams {
    offset: f32,
    z_distance: f32,
    k1: f32,
    k2: f32,
    sensitivity: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
    q_base: vec4f,    // [x, y, z, w]
    q_current: vec4f, // [x, y, z, w]
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> params: VRParams;

// 四元數共軛
fn q_conjugate(q: vec4f) -> vec4f {
    return vec4f(-q.xyz, q.w);
}

// 四元數乘法 (q1 * q2)
fn q_mul(q1: vec4f, q2: vec4f) -> vec4f {
    return vec4f(
        q1.w * q2.xyz + q2.w * q1.xyz + cross(q1.xyz, q2.xyz),
        q1.w * q2.w - dot(q1.xyz, q2.xyz)
    );
}

// 輔助函數：繞中心旋轉 UV (用於處理 Roll)
fn rotate_uv(uv: vec2f, angle: f32) -> vec2f {
    let s = sin(angle);
    let c = cos(angle);
    let pivot = vec2f(0.5, 0.5);
    let temp_uv = uv - pivot;
    let rotated = vec2f(
        temp_uv.x * c - temp_uv.y * s,
        temp_uv.x * s + temp_uv.y * c
    );
    return rotated + pivot;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2f, 6>(
        vec2f(-1.0, -1.0), vec2f( 1.0, -1.0), vec2f(-1.0,  1.0),
        vec2f(-1.0,  1.0), vec2f( 1.0, -1.0), vec2f( 1.0,  1.0)
    );
    
    let local_idx = vertex_index % 6u;
    let p = pos[local_idx];
    let is_right = f32(vertex_index >= 6u);

    // 輸出位置：SBS 佈局
    let screen_x = (p.x * 0.5) + (is_right * 1.0 - 0.5);
    let screen_y = p.y; 
    
    // 1. 計算相對旋轉 q_rel = q_current * inv(q_base)
    // 注意：若方向相反，請嘗試 q_mul(q_conjugate(params.q_base), params.q_current)
    let q = q_mul(params.q_current, q_conjugate(params.q_base));

    // 2. 從四元數提取歐拉角 (對應手機橫置 Z-Y-X 順序)
    // 根據你設定的：X=Roll, Y=Pitch, Z=Yaw
    
    // Yaw (繞 Z 軸)（左右看）
    let yaw = -atan2(2.0 * (q.w * q.z + q.x * q.y), 1.0 - 2.0 * (q.y * q.y + q.z * q.z));
    
    // Pitch (繞 Y 軸)(上下看)
    let pitch = -asin(clamp(2.0 * (q.w * q.y - q.z * q.x), -1.0, 1.0));
    
    // Roll (繞 X 軸)（
    let roll = atan2(2.0 * (q.w * q.x + q.y * q.z), 1.0 - 2.0 * (q.x * q.x + q.y * q.y));

    // 3. 計算基礎 UV 與視差
    let zoom = max(params.z_distance, 0.001);
    let parallax = (is_right - 0.5) * params.offset;
    
    var uv_x = (p.x * zoom + parallax + 1.0) / 2.0;
    var uv_y = 1.0 - ((p.y * zoom + 1.0) / 2.0);

    // 4. 套用旋轉位移
    // 這裡的 Yaw/Pitch 前面的正負號取決於你手機傳送端的座標系定義，若方向相反請翻轉
    uv_x = uv_x + (yaw * params.sensitivity);
    uv_y = uv_y - (pitch * params.sensitivity);

    // 5. 處理 Roll (歪頭時畫面要反向旋轉以保持地平線水平)
    var final_uv = vec2f(uv_x, uv_y);
    final_uv = rotate_uv(final_uv, roll); 

    var out: VertexOutput;
    out.position = vec4f(screen_x, screen_y, 0.0, 1.0);
    out.uv = final_uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let st = in.uv * 2.0 - 1.0;
    let r2 = st.x * st.x + st.y * st.y;

    let distortion = 1.0 + params.k1 * r2 + params.k2 * r2 * r2;
    let distorted_st = st * distortion;

    let final_uv = (distorted_st + 1.0) / 2.0;

    if (final_uv.x < 0.0 || final_uv.x > 1.0 || final_uv.y < 0.0 || final_uv.y > 1.0) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    return textureSample(screen_texture, screen_sampler, final_uv);
}