
pub fn generate_audio_2(audio_path: &Path, rate: f64) -> Result<()>{
    gst::init()?;
    let final_path = format!("{}({}).{}",audio_path.parent().unwrap().join(audio_path.file_stem().unwrap()).display(), rate, audio_path.extension().unwrap().to_str().unwrap());
    let audio_pipeline = Pipeline::with_name("ruso_generate_audio");
    let filesrc = ElementFactory::make("filesrc").property("location", audio_path.to_str().unwrap()).build()?;
    let filesink = ElementFactory::make("filesink").property("location", final_path.as_str()).build()?;
    dbg!(&final_path.as_str());

    match audio_path.extension().unwrap().to_str().unwrap().to_lowercase().as_str(){
        "mp3" =>{
            dbg!("working...");
            let mpegaudioparse = ElementFactory::make("mpegaudioparse").build()?;
            let mpg123audiodec = ElementFactory::make("mpg123audiodec").build()?;
            let decodebin = ElementFactory::make("decodebin").build()?;
            let audioconvert = ElementFactory::make("audioconvert").build()?;
            let audioconvert2 = ElementFactory::make("audioconvert").build()?;
            let audioresample = ElementFactory::make("audioresample").build()?;
            let audioresample2 = ElementFactory::make("audioresample").build()?;
            let speed = ElementFactory::make("speed").property("speed", rate as f32).build()?;
            let lamemp3enc = ElementFactory::make("lamemp3enc").property("quality", 0.0f32).build()?;
            let id3v2mux = ElementFactory::make("id3v2mux").build()?;
            audio_pipeline.add_many([&filesrc, &mpegaudioparse, &mpg123audiodec, &decodebin, &audioconvert, &audioresample, &speed, &audioconvert2, &audioresample2, &lamemp3enc, &id3v2mux, &filesink])?;
            // filesink.link(&mpegaudioparse)?;
            // mpegaudioparse.link(&mpg123audiodec)?;
            // mpg123audiodec.link(&decodebin)?;
            // decodebin.link(&audioconvert)?;
            // audioconvert.link(&audioresample)?;
            // audioresample.link(&speed)?;
            // speed.link(&audioconvert2)?;
            // audioconvert2.link(&audioresample2)?;
            // audioresample2.link(&lamemp3enc)?;
            // lamemp3enc.link(&id3v2mux)?;
            // id3v2mux.link(&filesink)?;
            gst::Element::link(&filesrc, &mpegaudioparse)?;
            gst::Element::link_many([&mpg123audiodec, &decodebin, &audioconvert, &audioresample, &speed, &audioconvert2, &audioresample2, &lamemp3enc])?;
            gst::Element::link(&id3v2mux, &filesink)?;

            // let bus = audio_pipeline.bus().unwrap();
            // bus.add_signal_watch();
            // let mut bus_stream = bus.stream();
            // audio_pipeline.set_state(gst::State::Playing)?;
            // println!("{:?}", bus_stream.next().await);
            // while let Some(message) = bus_stream.next().await {
            //     println!("new message [{:?}]", message.type_());
            //     println!("balls");
            //     match message.view() {
            //         MessageView::Eos(_) => {
            //             break;
            //         }
            //         MessageView::Error(e) => {
            //             audio_pipeline.set_state(gst::State::Null)?;
            //             return Err(anyhow::anyhow!("Error from {:?}: {} ({:?})", message.src().map(|s| s.path_string()), e.error(), e.debug()));
            //         }
            //         _ => (),
            //     }
            // }
            audio_pipeline.set_state(gst::State::Playing)?;
            let bus = audio_pipeline.bus().unwrap();
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                dbg!(&msg);
                match msg.view() {
                    MessageView::Eos(..) => break,
                    MessageView::Error(err) => {
                        println!(
                            "Error from {:?}: {} ({:?})",
                            err.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug()
                        );
                        break;
                    }
                    m => println!("{:?}", m)
                }
            }
        },
        "ogg" =>{
            let oggdemux = ElementFactory::make("oggdemux").build()?;
            let vorbisdec = ElementFactory::make("vorbisdec").build()?;
            let audioconvert = ElementFactory::make("audioconvert").build()?;
            let speed = ElementFactory::make("speed").build()?;
            speed.set_property("speed", rate as f32);
            let vorbisenc = ElementFactory::make("vorbisenc").build()?;
            let oggmux = ElementFactory::make("oggmux").build()?;
            audio_pipeline.add_many([&filesrc, &oggdemux, &vorbisdec, &audioconvert, &speed, &vorbisenc, &oggmux, &filesink])?;
            audio_pipeline.set_state(gst::State::Playing)?;
            let bus = audio_pipeline.bus().unwrap();
            bus.add_signal_watch();
            loop {
                if bus.pop().is_some_and(|x| x.as_ref().type_().eq(&MessageType::Eos)){
                    break;
                }
            }
        },
        "wav" =>{
            let wavparse = ElementFactory::make("wavparse").build()?;
            let audioconvert = ElementFactory::make("audioconvert").build()?;
            let audioresample = ElementFactory::make("audioresample").build()?;
            let speed = ElementFactory::make("speed").build()?;
            speed.set_property("speed", rate as f32);
            let wavenc = ElementFactory::make("wavenc").build()?;
            audio_pipeline.add_many([&filesrc, &wavparse, &audioconvert, &audioresample, &speed, &audioconvert, &wavenc, &filesink])?;
            audio_pipeline.set_state(gst::State::Playing)?;
            let bus = audio_pipeline.bus().unwrap();
            bus.add_signal_watch();
            loop {
                if bus.pop().is_some_and(|x| x.as_ref().type_().eq(&MessageType::Eos)){
                    break;
                }
            }
        },
        e => return Err(anyhow::anyhow!("Unsupported file type: {}", e))
    }
    audio_pipeline.set_state(gst::State::Null)?;
    Ok(())
}
