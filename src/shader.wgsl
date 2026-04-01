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
    @location(0) view_dir: vec3f, // 傳遞 3D 視線向量到 Fragment
    @location(1) is_right: f32,
};

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> params: VRParams;

// --- 四元數運算工具函數 ---

fn q_conjugate(q: vec4f) -> vec4f {
    return vec4f(-q.xyz, q.w);
}

fn q_mul(q1: vec4f, q2: vec4f) -> vec4f {
    return vec4f(
        q1.w * q2.xyz + q2.w * q1.xyz + cross(q1.xyz, q2.xyz),
        q1.w * q2.w - dot(q1.xyz, q2.xyz)
    );
}

// 使用四元數旋轉 3D 向量 (Rodrigues' Rotation Formula 的優化版)
fn rotate_vector(v: vec3f, q: vec4f) -> vec3f {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2f, 6>(
        vec2f(-1.0, -1.0), vec2f( 1.0, -1.0), vec2f(-1.0,  1.0),
        vec2f(-1.0,  1.0), vec2f( 1.0, -1.0), vec2f( 1.0,  1.0)
    );
    
    let local_idx = vertex_index % 6u;
    let p = pos[local_idx];
    let is_right_val = f32(vertex_index >= 6u);

    // 1. 決定在螢幕上的位置 (SBS 佈局)
    let screen_x = (p.x * 0.5) + (is_right_val * 1.0 - 0.5);
    let screen_y = p.y; 

    // 2. 建立「原始視線向量」
    // z_distance 代表焦距，數值越大視場 (FOV) 越窄
    let parallax = (is_right_val - 0.5) * params.offset;
    var raw_view_dir = vec3f(p.x + parallax, p.y, params.z_distance);

    // qflipZ​=(0,0,1,0), q' = qflipZ​⋅q⋅qflipZ−1​
    // let q_flip_z = vec4f(1, 1, 0, 0);


    // 3. 計算相對旋轉四元數 並 取其逆旋轉 (Conjugate)
    // q_rel = q_current * inv(q_base)
    // 逆旋轉代表：相機轉向左邊，畫面採樣點要向右旋轉
    let q_rel = q_mul(q_conjugate(params.q_base), params.q_current);
    // let q_rel_fix = q_mul(q_mul(q_flip_z, q_rel), q_conjugate(q_flip_z));

    // 4. 旋轉視線向量
    let q_cam_inv = vec4f(-q_rel[0],-q_rel[1],q_rel[2],q_rel[3]);// 鏡頭旋轉是(z)反過來的
    let rotated_dir = rotate_vector(raw_view_dir, q_cam_inv);

    var out: VertexOutput;
    out.position = vec4f(screen_x, screen_y, 0.0, 1.0);
    out.view_dir = rotated_dir;
    out.is_right = is_right_val;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // 1. 將旋轉後的 3D 向量重新投影回 2D 平面 (透視投影)
    // rotated_dir.z 是視線前方的深度
    var proj_uv = vec2f(in.view_dir.x / in.view_dir.z, in.view_dir.y / in.view_dir.z);
    
    // 2. 轉回 [0, 1] UV 空間
    // 注意：因原始畫面是 Y-up，UV 座標通常需要翻轉 Y
    let st = proj_uv; // 現在是中心為 0 的座標系 [-1, 1] (視 FOV 而定)
    
    // 3. 桶狀畸變 (Barrel Distortion) 
    // 在投影座標上進行，效果最精確
    let r2 = dot(st, st);
    let distortion = 1.0 + params.k1 * r2 + params.k2 * r2 * r2;
    let distorted_st = st * distortion;

    // 4. 轉換為最終採樣 UV
    let final_uv = vec2f(
        (distorted_st.x + 1.0) / 2.0,
        1.0 - (distorted_st.y + 1.0) / 2.0
    );

    // 邊界檢查
    if (final_uv.x < 0.0 || final_uv.x > 1.0 || final_uv.y < 0.0 || final_uv.y > 1.0) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }

    return textureSample(screen_texture, screen_sampler, final_uv);
}