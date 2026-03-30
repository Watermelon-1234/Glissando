use gstreamer as gst;
use gstreamer_app as gst_app;
use gst::prelude::*;
use anyhow::Result;

pub struct GStreamer {
    pipeline: gst::Pipeline,
    appsrc: gst_app::AppSrc,
}

impl GStreamer {
    pub fn new(target_ip: &str, target_port: u16, width: u32, height: u32) -> Result<Self> {
        gst::init()?;

        // 跨平台 Pipeline 字串
        // appsrc: 接收外部數據
        // videoconvert: 確保格式正確
        // x264enc: 通用 H.264 編碼 (tune=zerolatency 是關鍵)
        // rtph264pay: 轉成 RTP 封包
        // udpsink: 噴向目標

        let encoder = if cfg!(target_os = "macos") {
            // 移除 usage=lowlatency，改用更基礎但有效的參數
            "vtenc_h264 allow-frame-reordering=false realtime=true max-keyframe-interval=2"
        } else if cfg!(target_os = "windows") {
            // 關鍵：加入 rc-mode=2 (CBR), bitrate=4000 (限制碼率), zerolatency=true
            "nvh264enc preset=low-latency-hq gop=2 zerolatency=true rc-mode=2 bitrate=4000"
        } else {
            "x264enc tune=zerolatency speed-preset=ultrafast key-int-max=2"
        };

        println!("target_ip: {}", target_ip);

        let pipeline_str = format!(
            "appsrc name=src is-live=true do-timestamp=true min-latency=0 max-latency=0 ! \
            video/x-raw,format=I420,width={width},height={height},framerate=60/1 ! \
            videoconvert ! {encoder} ! \
            h264parse ! \
            mpegtsmux alignment=1 ! \
            udpsink host={target_ip} port={target_port} sync=false async=false"
        );

        let pipeline = gst::parse::launch(&pipeline_str)?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast pipeline"))?;

        let appsrc = pipeline
            .by_name("src")
            .ok_or_else(|| anyhow::anyhow!("Source element not found"))?
            .downcast::<gst_app::AppSrc>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast appsrc"))?;

        // 設為流模式
        appsrc.set_format(gst::Format::Time);
        appsrc.set_is_live(true);

        pipeline.set_state(gst::State::Playing)?;

        Ok(Self { pipeline, appsrc })
    }

    pub fn push_frame(&self, data: Vec<u8>) -> Result<(), anyhow::Error> {
        let buffer = gst::Buffer::from_mut_slice(data);
        match self.appsrc.push_buffer(buffer) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}