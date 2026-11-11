use std::path::Path;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Variant::VT_I8;
use windows::core::{GUID, HSTRING};

pub const TICKS: f64 = 10_000_000.0;
pub const OUT_RATE: u32 = 48_000;

fn err(e: windows::core::Error) -> String {
    e.message().to_string()
}

pub fn startup() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _ = MFStartup(MF_VERSION, MFSTARTUP_FULL);
    }
}

fn reader(path: &Path) -> Result<IMFSourceReader, String> {
    startup();
    unsafe {
        let mut attrs: Option<IMFAttributes> = None;
        MFCreateAttributes(&mut attrs, 2).map_err(err)?;
        let attrs = attrs.ok_or("attributs MF")?;
        attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING, 1).map_err(err)?;
        MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attrs).map_err(err)
    }
}

fn duration(r: &IMFSourceReader) -> f64 {
    unsafe {
        r.GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
            .map(|pv| pv.Anonymous.Anonymous.Anonymous.uhVal as f64 / TICKS)
            .unwrap_or(0.0)
    }
}

fn seek(r: &IMFSourceReader, secs: f64) -> Result<(), String> {
    unsafe {
        let mut pv = PROPVARIANT::default();
        (*pv.Anonymous.Anonymous).vt = VT_I8;
        (*pv.Anonymous.Anonymous).Anonymous.hVal = (secs.max(0.0) * TICKS) as i64;
        r.SetCurrentPosition(&GUID::zeroed(), &pv).map_err(err)
    }
}

fn read_bytes(sample: &IMFSample) -> Result<Vec<u8>, String> {
    unsafe {
        let buf = sample.ConvertToContiguousBuffer().map_err(err)?;
        let mut ptr = std::ptr::null_mut();
        let mut len = 0u32;
        buf.Lock(&mut ptr, None, Some(&mut len)).map_err(err)?;
        let out = std::slice::from_raw_parts(ptr, len as usize).to_vec();
        buf.Unlock().map_err(err)?;
        Ok(out)
    }
}

pub struct Info {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub has_video: bool,
    pub has_audio: bool,
}

pub struct VideoReader {
    r: IMFSourceReader,
    pub width: u32,
    pub height: u32,
    pub last: Option<(f64, Vec<u8>)>,
    eof: bool,
}

impl VideoReader {
    pub fn open(path: &Path) -> Result<(Self, Info), String> {
        let r = reader(path)?;
        unsafe {
            r.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false).map_err(err)?;
            let has_video = r.SetStreamSelection(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, true).is_ok();
            let has_audio = r.GetCurrentMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32).is_ok();
            let (mut w, mut h, mut fps) = (0, 0, 30.0);
            if has_video {
                let mt = MFCreateMediaType().map_err(err)?;
                mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(err)?;
                mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(err)?;
                r.SetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, None, &mt).map_err(err)?;
                let cur = r.GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32).map_err(err)?;
                let size = cur.GetUINT64(&MF_MT_FRAME_SIZE).map_err(err)?;
                w = (size >> 32) as u32;
                h = size as u32;
                if let Ok(rate) = cur.GetUINT64(&MF_MT_FRAME_RATE) {
                    let (n, d) = ((rate >> 32) as f64, (rate as u32).max(1) as f64);
                    if n > 0.0 {
                        fps = n / d;
                    }
                }
            }
            let info = Info { duration: duration(&r), width: w, height: h, fps, has_video, has_audio };
            Ok((Self { r, width: w, height: h, last: None, eof: !has_video }, info))
        }
    }

    pub fn next(&mut self) -> Result<Option<(f64, Vec<u8>)>, String> {
        if self.eof {
            return Ok(None);
        }
        unsafe {
            loop {
                let mut flags = 0u32;
                let mut ts = 0i64;
                let mut sample: Option<IMFSample> = None;
                self.r
                    .ReadSample(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut sample))
                    .map_err(err)?;
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    self.eof = true;
                    return Ok(None);
                }
                if let Some(s) = sample {
                    let bgra = read_bytes(&s)?;
                    let expected = (self.width * self.height * 4) as usize;
                    if bgra.len() < expected {
                        continue;
                    }
                    let mut rgba = bgra[..expected].to_vec();
                    for px in rgba.chunks_exact_mut(4) {
                        px.swap(0, 2);
                        px[3] = 255;
                    }
                    let f = (ts as f64 / TICKS, rgba);
                    self.last = Some(f.clone());
                    return Ok(Some(f));
                }
            }
        }
    }

    pub fn frame_at(&mut self, t: f64, fps: f64) -> Result<Option<&(f64, Vec<u8>)>, String> {
        let half = 0.5 / fps.max(1.0);
        let need_seek = match &self.last {
            Some((lt, _)) => t + half < *lt || t > lt + 1.0,
            None => true,
        };
        if need_seek {
            seek(&self.r, t)?;
            self.eof = false;
            self.last = None;
        }
        loop {
            if let Some((lt, _)) = &self.last
                && *lt + half > t
            {
                break;
            }
            if self.next()?.is_none() {
                break;
            }
        }
        Ok(self.last.as_ref())
    }
}

pub fn decode_audio(path: &Path) -> Result<Vec<f32>, String> {
    let r = reader(path)?;
    unsafe {
        r.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false).map_err(err)?;
        if r.SetStreamSelection(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32, true).is_err() {
            return Ok(Vec::new());
        }
        let mt = MFCreateMediaType().map_err(err)?;
        mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(err)?;
        mt.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_Float).map_err(err)?;
        mt.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, OUT_RATE).map_err(err)?;
        mt.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2).map_err(err)?;
        mt.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 32).map_err(err)?;
        mt.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 8).map_err(err)?;
        mt.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, OUT_RATE * 8).map_err(err)?;
        r.SetCurrentMediaType(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32, None, &mt).map_err(err)?;
        let mut out = Vec::new();
        loop {
            let mut flags = 0u32;
            let mut sample: Option<IMFSample> = None;
            r.ReadSample(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32, 0, None, Some(&mut flags), None, Some(&mut sample)).map_err(err)?;
            if let Some(s) = sample {
                let bytes = read_bytes(&s)?;
                out.extend(bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])));
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                break;
            }
        }
        Ok(out)
    }
}

pub struct Encoder {
    w: IMFSinkWriter,
    width: u32,
    height: u32,
    fps: u32,
    frame: u64,
    audio_pos: u64,
}

impl Encoder {
    pub fn new(path: &Path, width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self, String> {
        startup();
        unsafe {
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 1).map_err(err)?;
            let attrs = attrs.ok_or("attributs MF")?;
            attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1).map_err(err)?;
            let w = MFCreateSinkWriterFromURL(&HSTRING::from(path.as_os_str()), None, &attrs).map_err(err)?;

            let size = ((width as u64) << 32) | height as u64;
            let rate = ((fps as u64) << 32) | 1;
            let vout = MFCreateMediaType().map_err(err)?;
            vout.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(err)?;
            vout.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).map_err(err)?;
            vout.SetUINT32(&MF_MT_AVG_BITRATE, bitrate).map_err(err)?;
            vout.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(err)?;
            vout.SetUINT64(&MF_MT_FRAME_SIZE, size).map_err(err)?;
            vout.SetUINT64(&MF_MT_FRAME_RATE, rate).map_err(err)?;
            vout.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1).map_err(err)?;
            let vs = w.AddStream(&vout).map_err(err)?;
            let vin = MFCreateMediaType().map_err(err)?;
            vin.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(err)?;
            vin.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(err)?;
            vin.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(err)?;
            vin.SetUINT64(&MF_MT_FRAME_SIZE, size).map_err(err)?;
            vin.SetUINT64(&MF_MT_FRAME_RATE, rate).map_err(err)?;
            vin.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1).map_err(err)?;
            vin.SetUINT32(&MF_MT_DEFAULT_STRIDE, width * 4).map_err(err)?;
            w.SetInputMediaType(vs, &vin, None).map_err(err)?;

            let aout = MFCreateMediaType().map_err(err)?;
            aout.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(err)?;
            aout.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC).map_err(err)?;
            aout.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).map_err(err)?;
            aout.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, OUT_RATE).map_err(err)?;
            aout.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2).map_err(err)?;
            aout.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 24_000).map_err(err)?;
            let as_ = w.AddStream(&aout).map_err(err)?;
            let ain = MFCreateMediaType().map_err(err)?;
            ain.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(err)?;
            ain.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM).map_err(err)?;
            ain.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).map_err(err)?;
            ain.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, OUT_RATE).map_err(err)?;
            ain.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2).map_err(err)?;
            ain.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4).map_err(err)?;
            ain.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, OUT_RATE * 4).map_err(err)?;
            w.SetInputMediaType(as_, &ain, None).map_err(err)?;

            w.BeginWriting().map_err(err)?;
            Ok(Self { w, width, height, fps, frame: 0, audio_pos: 0 })
        }
    }

    fn sample(bytes: &[u8], time: i64, dur: i64) -> Result<IMFSample, String> {
        unsafe {
            let buf = MFCreateMemoryBuffer(bytes.len() as u32).map_err(err)?;
            let mut ptr = std::ptr::null_mut();
            buf.Lock(&mut ptr, None, None).map_err(err)?;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            buf.Unlock().map_err(err)?;
            buf.SetCurrentLength(bytes.len() as u32).map_err(err)?;
            let s = MFCreateSample().map_err(err)?;
            s.AddBuffer(&buf).map_err(err)?;
            s.SetSampleTime(time).map_err(err)?;
            s.SetSampleDuration(dur).map_err(err)?;
            Ok(s)
        }
    }

    pub fn video(&mut self, rgba: &[u8]) -> Result<(), String> {
        let len = (self.width * self.height * 4) as usize;
        let mut bgra = Vec::with_capacity(len);
        for px in rgba[..len].chunks_exact(4) {
            bgra.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
        let dur = (TICKS / self.fps as f64) as i64;
        let s = Self::sample(&bgra, self.frame as i64 * dur, dur)?;
        unsafe { self.w.WriteSample(0, &s).map_err(err)? };
        self.frame += 1;
        Ok(())
    }

    pub fn audio(&mut self, interleaved: &[f32]) -> Result<(), String> {
        if interleaved.is_empty() {
            return Ok(());
        }
        let bytes: Vec<u8> = interleaved.iter().flat_map(|v| ((v.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
        let frames = (interleaved.len() / 2) as u64;
        let time = (self.audio_pos as f64 / OUT_RATE as f64 * TICKS) as i64;
        let dur = (frames as f64 / OUT_RATE as f64 * TICKS) as i64;
        let s = Self::sample(&bytes, time, dur)?;
        unsafe { self.w.WriteSample(1, &s).map_err(err)? };
        self.audio_pos += frames;
        Ok(())
    }

    pub fn finish(self) -> Result<(), String> {
        unsafe { self.w.Finalize().map_err(err) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_mp4() {
        let path = std::env::temp_dir().join("ethone-cut-test.mp4");
        let _ = std::fs::remove_file(&path);
        let (w, h, fps) = (320u32, 240u32, 30u32);
        let mut enc = Encoder::new(&path, w, h, fps, 2_000_000).unwrap();
        for f in 0..60 {
            let mut rgba = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    let i = ((y * w + x) * 4) as usize;
                    let c = if y < h / 2 { [230, 30, 30] } else { [30, 30, 230] };
                    rgba[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
                }
            }
            enc.video(&rgba).unwrap();
            let tone: Vec<f32> = (0..OUT_RATE / fps)
                .flat_map(|i| {
                    let v = ((f * (OUT_RATE / fps) + i) as f32 * 440.0 * std::f32::consts::TAU / OUT_RATE as f32).sin() * 0.3;
                    [v, v]
                })
                .collect();
            enc.audio(&tone).unwrap();
        }
        enc.finish().unwrap();

        let (mut r, info) = VideoReader::open(&path).unwrap();
        assert!(info.has_video && info.has_audio);
        assert_eq!((info.width, info.height), (w, h));
        assert!((info.duration - 2.0).abs() < 0.2, "durée {}", info.duration);
        let (_, frame) = r.frame_at(1.0, info.fps).unwrap().unwrap().clone();
        let top = &frame[((20 * w + 160) * 4) as usize..][..3];
        let bottom = &frame[((220 * w + 160) * 4) as usize..][..3];
        assert!(top[0] > 150 && top[2] < 100, "haut {top:?}");
        assert!(bottom[2] > 150 && bottom[0] < 100, "bas {bottom:?}");

        let audio = decode_audio(&path).unwrap();
        assert!(audio.len() > OUT_RATE as usize * 2 * 18 / 10);
        let peak = audio.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.2 && peak < 0.5, "pic {peak}");
        let _ = std::fs::remove_file(&path);
    }
}
