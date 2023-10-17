use std::{path::PathBuf, fs::File, num::NonZeroU32, io::Write};
use anyhow::{Result, anyhow};
use id3::{Tag, TagLike};
use mp3lame_encoder::{FlushNoGap, Id3Tag, InterleavedPcm, max_required_buffer_size, MonoPcm};
use vorbis_rs::{VorbisDecoder, VorbisEncoderBuilder};
use soundtouch::SoundTouch;

pub fn change_speed_wav(path: &PathBuf, rate: f64, change_pitch: bool) -> Result<(), hound::Error>{
    let mut reader = hound::WavReader::open(path)?;
    let mut spec = hound::WavSpec{
        ..reader.spec()
    };

    let samples = reader.samples::<i16>().map(|x| x.unwrap() as f32).collect::<Vec<f32>>();
    let out_data: Vec<f32>;

    if change_pitch{
        spec.sample_rate = (spec.sample_rate as f64 * rate) as u32;
        out_data = samples;
    }else{
        let mut soundtouch = SoundTouch::new();
        soundtouch
            .set_sample_rate(reader.spec().sample_rate)
            .set_channels(reader.spec().channels as u32)
            .set_tempo(rate);
            out_data = soundtouch.generate_audio(&samples);
    }

    let mut encoder = hound::WavWriter::create(format!("{}({})", path.display(), rate), spec)?;

    for sample in out_data{
        encoder.write_sample(sample)?;
    };
    encoder.finalize()?;
    Ok(())
}

pub fn change_speed_ogg(path: &PathBuf, rate: f64, change_pitch: bool) -> Result<()>{
    let mut source_ogg = File::open(path)?;
    let mut transcoded_ogg = Vec::new();
    let mut decoder = VorbisDecoder::new(&mut source_ogg)?;
    let mut encoder = VorbisEncoderBuilder::new(
        NonZeroU32::new((decoder.sampling_frequency().get() as f64 * rate) as u32).unwrap(),
        decoder.channels(),
        &mut transcoded_ogg
    )?.build()?;

    while let Some(decoded_block) = decoder.decode_audio_block()? {
        encoder.encode_audio_block(decoded_block.samples())?;
    }

    encoder.finish()?;
    File::create(format!("{}({}).ogg", path.parent().unwrap().join(path.file_stem().unwrap()).display(), rate))?.write_all(transcoded_ogg.as_slice())?;
    Ok(())
}

pub fn change_speed_mp3(path: &PathBuf, rate: f64, change_pitch: bool) -> Result<()>{
    let mut mp3_data: Vec<u8> = Vec::new();
    let mut decoder = minimp3::Decoder::new(File::open(path)?);
    let tag = Tag::read_from_path(path);
    let mp3_headers = decoder.next_frame()?;
    let mut encoder = mp3lame_encoder::Builder::new().ok_or(anyhow!("Could not instantiate an mp3 builder"))?;

    encoder.set_num_channels(mp3_headers.channels as u8).map_err(|e| anyhow!("Could not set mp3 encoder channels: {}", e))?;
    encoder.set_quality(mp3lame_encoder::Quality::Best).map_err(|e| anyhow!("Could not set mp3 encoder quality: {}", e))?;
    encoder.set_mode(mp3lame_encoder::Mode::Stereo).map_err(|e| anyhow!("Could not set mp3 audio mode: {}", e))?;

    encoder.set_brate(match mp3_headers.bitrate {
        _ if mp3_headers.bitrate >= 320 => mp3lame_encoder::Bitrate::Kbps320,
        _ if mp3_headers.bitrate >= 256 => mp3lame_encoder::Bitrate::Kbps256,
        _ if mp3_headers.bitrate >= 224 => mp3lame_encoder::Bitrate::Kbps224,
        _ if mp3_headers.bitrate >= 192 => mp3lame_encoder::Bitrate::Kbps192,
        _ if mp3_headers.bitrate >= 160 => mp3lame_encoder::Bitrate::Kbps160,
        _ if mp3_headers.bitrate >= 128 => mp3lame_encoder::Bitrate::Kbps128,
        _ if mp3_headers.bitrate >= 112 => mp3lame_encoder::Bitrate::Kbps112,
        _ if mp3_headers.bitrate >= 96  => mp3lame_encoder::Bitrate::Kbps96,
        _ if mp3_headers.bitrate >= 80  => mp3lame_encoder::Bitrate::Kbps80,
        _ if mp3_headers.bitrate >= 64  => mp3lame_encoder::Bitrate::Kbps64,
        _ if mp3_headers.bitrate >= 48  => mp3lame_encoder::Bitrate::Kbps48,
        _ if mp3_headers.bitrate >= 40  => mp3lame_encoder::Bitrate::Kbps40,
        _ if mp3_headers.bitrate >= 32  => mp3lame_encoder::Bitrate::Kbps32,
        _ if mp3_headers.bitrate >= 24  => mp3lame_encoder::Bitrate::Kbps24,
        _ if mp3_headers.bitrate >= 16  => mp3lame_encoder::Bitrate::Kbps16,
        _ if mp3_headers.bitrate >= 8   => mp3lame_encoder::Bitrate::Kbps8,
        _ => mp3lame_encoder::Bitrate::Kbps96,
    }).map_err(|e| anyhow!("Could not set mp3 bitrate: {}", e))?;

    if let Ok(tag) = &tag{
        let year: Box<[u8]> = if let Some(year) = tag.year(){
            year.to_string().as_bytes().into()
        }else{
            b"".as_slice().into()
        };
        encoder.set_id3_tag(Id3Tag{
            album: tag.album().unwrap_or("").as_bytes(),
            artist: tag.artist().unwrap_or("").as_bytes(),
            comment: if let Some(comment) = tag.comments().next() {comment.text.as_bytes()} else {b""},
            title: tag.title().unwrap_or("").as_bytes(),
            year: year.as_ref()
        });
    }
    
    let len = mp3_headers.data.len();
    let mut input: Vec<i16> = mp3_headers.data;
    input.reserve(len * 3000);
    while let Ok(mut frame) = decoder.next_frame() {
        input.append(&mut frame.data);
    }

    encoder.set_sample_rate(
        if change_pitch{
            (mp3_headers.sample_rate as f64 * rate) as u32
        }else{
            let mut soundtouch = SoundTouch::new();
            soundtouch
                .set_sample_rate(mp3_headers.sample_rate as u32)
                .set_channels(mp3_headers.channels as u32)
                .set_tempo(rate);
                input = soundtouch.generate_audio(input.into_iter().map(|x| x as f32).collect::<Vec<f32>>().as_slice())
                    .into_iter().map(|x| x as i16).collect::<Vec<i16>>();
            mp3_headers.sample_rate as u32
        }
    ).map_err(|e| anyhow!("Could not set mp3 sample rate: {}", e))?;

    let mut encoder = encoder.build().map_err(|e| anyhow!("Could not build mp3 encoder: {}", e))?;
    if encoder.num_channels() == 1 {
        let input = MonoPcm(&input);
        mp3_data.reserve(max_required_buffer_size(input.0.len()));
        let encoded_size = encoder.encode(input, mp3_data.spare_capacity_mut()).map_err(|e| anyhow!(e))?;
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
        let encoded_size = encoder.flush::<FlushNoGap>(mp3_data.spare_capacity_mut()).map_err(|e| anyhow!(e))?;
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
    }else{
        let input = InterleavedPcm(&input);
        mp3_data.reserve(max_required_buffer_size(input.0.len()));
        let encoded_size = encoder.encode(input, mp3_data.spare_capacity_mut()).map_err(|e| anyhow!(e))?;
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
        let encoded_size = encoder.flush::<FlushNoGap>(mp3_data.spare_capacity_mut()).map_err(|e| anyhow!(e))?;
        
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
    } 

    let out_path = PathBuf::from(format!("{}({}).mp3", path.parent().unwrap().join(path.file_stem().unwrap()).display(), rate));
    let mut file = File::create(&out_path)?;
    file.write_all(mp3_data.as_slice())?;
    if let Ok(tag) = tag {
        tag.write_to_path(out_path, id3::Version::Id3v24)?;
    }

    Ok(())
}
