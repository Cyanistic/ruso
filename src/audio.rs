use std::{path::PathBuf, fs::File, num::NonZeroU32, io::Write};
use anyhow::Result;
use id3::{Tag, TagLike};
use mp3lame_encoder::{FlushNoGap, Id3Tag};
use std::mem::MaybeUninit;
use vorbis_rs::{VorbisDecoder, VorbisEncoderBuilder};

pub fn change_speed_wav(path: &PathBuf, rate: f64) -> Result<(), hound::Error>{
    let mut reader = hound::WavReader::open(path)?;
    let sample_rate = reader.spec().sample_rate;
    let channels = reader.spec().channels;
    let spec = hound::WavSpec{
        channels,
        sample_rate: (sample_rate as f64 * rate) as u32,
        bits_per_sample: reader.spec().bits_per_sample,
        sample_format: reader.spec().sample_format,
    };
    let mut writer = hound::WavWriter::create(format!("{}({})", path.display(), rate), spec)?;
    for sample in reader.samples::<i16>(){
        writer.write_sample(sample?)?;
    };
    writer.finalize()?;
    Ok(())
}

pub fn change_speed_ogg(path: &PathBuf, rate: f64) -> Result<()>{
    let mut source_ogg = File::open(path)?;
    // let mut transcoded_ogg = File::create(format!("{}({})", path.display(), rate))?;
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

pub fn change_speed_mp3(path: &PathBuf, rate: f64) -> Result<()>{
    let mut mp3_data: Vec<MaybeUninit<u8>> = Vec::new();
    let mut decoder = minimp3::Decoder::new(File::open(path)?);
    let tag = Tag::read_from_path(path);
    let mp3_headers = decoder.next_frame()?;
    let mut encoder = mp3lame_encoder::Builder::new().unwrap();
    encoder.set_sample_rate((mp3_headers.sample_rate as f64 * rate) as u32).unwrap();
    encoder.set_num_channels(mp3_headers.channels as u8).unwrap();
    encoder.set_quality(mp3lame_encoder::Quality::Best).unwrap();
    encoder.set_mode(mp3lame_encoder::Mode::Stereo).unwrap();

    match mp3_headers.bitrate {
        _ if mp3_headers.bitrate >= 320 => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps320).unwrap(),
        _ if mp3_headers.bitrate >= 256 => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps256).unwrap(),
        _ if mp3_headers.bitrate >= 224 => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps224).unwrap(),
        _ if mp3_headers.bitrate >= 192 => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps192).unwrap(),
        _ if mp3_headers.bitrate >= 160 => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps160).unwrap(),
        _ if mp3_headers.bitrate >= 128 => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps128).unwrap(),
        _ if mp3_headers.bitrate >= 112 => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps112).unwrap(),
        _ if mp3_headers.bitrate >= 96  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps96).unwrap(),
        _ if mp3_headers.bitrate >= 80  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps80).unwrap(),
        _ if mp3_headers.bitrate >= 64  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps64).unwrap(),
        _ if mp3_headers.bitrate >= 48  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps48).unwrap(),
        _ if mp3_headers.bitrate >= 40  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps40).unwrap(),
        _ if mp3_headers.bitrate >= 32  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps32).unwrap(),
        _ if mp3_headers.bitrate >= 24  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps24).unwrap(),
        _ if mp3_headers.bitrate >= 16  => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps16).unwrap(),
        _ if mp3_headers.bitrate >= 8   => encoder.set_brate(mp3lame_encoder::Bitrate::Kbps8).unwrap(),
        _ => ()
    };

    if let Ok(tag) = tag{
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
    
    let mut encoder = encoder.build().unwrap();
    let len = mp3_headers.data.len();
    let mut input: Vec<i16> = mp3_headers.data;
    input.reserve(len * 3000);
    while let Ok(mut frame) = decoder.next_frame() {
        input.append(&mut frame.data);
    }

    if encoder.num_channels() == 1 {
        let input = mp3lame_encoder::MonoPcm(&input);
        mp3_data.reserve(mp3lame_encoder::max_required_buffer_size(input.0.len()));
        let encoded_size = encoder.encode(input, &mut mp3_data).unwrap();
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
        let encoded_size = encoder.flush::<FlushNoGap>(&mut mp3_data).unwrap();
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
    }else{
        let input = mp3lame_encoder::InterleavedPcm(&input);
        mp3_data.reserve(mp3lame_encoder::max_required_buffer_size(input.0.len()));
        let encoded_size = encoder.encode(input, &mut mp3_data).unwrap();
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
        let encoded_size = encoder.flush::<FlushNoGap>(&mut mp3_data).unwrap();
        unsafe {
            mp3_data.set_len(mp3_data.len().wrapping_add(encoded_size));
        }
    } 

    let mp3_data: Vec<u8> = unsafe {
        std::mem::transmute(mp3_data)
    };
    File::create(format!("{}({}).mp3", path.parent().unwrap().join(path.file_stem().unwrap()).display(), rate))?.write_all(mp3_data.as_slice())?;

    Ok(())
}
