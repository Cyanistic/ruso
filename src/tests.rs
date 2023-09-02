#[cfg(test)]
mod test{
    use super::*;
    // #[tokio::test]
    // async fn test1(){
    //     generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/991895 Kondo Koji - Slider/Kondo Koji - Slider (NikoSek) [YaHoo!!].osu"), 1.9).await.unwrap();
    // }
    // #[tokio::test]
    // async fn test2(){
    //     generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/1869337 Fellowship - Glory Days/Fellowship - Glory Days (EdgyKing) [Selfless Journey].osu"), 3.0).await.unwrap();
    // }
    #[tokio::test]
    async fn gosu_test(){
        gosu_websocket_listen(&Settings::new()).await.unwrap();
    }

    #[tokio::test]
    async fn audio_test(){
        generate_audio(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/1941727 Mrs GREEN APPLE - StaRt (Katagiri Remix)/audio.mp3"), 1.7);
    }

    #[tokio::test]
    async fn metadata_test(){
        read_map_metadata(MapOptions
            { approach_rate: 5.0,
            circle_size: 5.0,
            hp_drain: 5.0,
            overall_difficulty: 5.0,
            background: None,
            map_path: PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs"),
            bpm: 100,
            rate: 1.3,
            artist: "".into(),
            title: "".into(),
            difficulty_name: "".into()
        }, &Settings::new()).unwrap();
    }

    #[tokio::test]
    async fn get_bpm(){
        let map = libosu::beatmap::Beatmap::parse(File::open("/home/cyan/.local/share/osu-wine/osu!/Songs/991895 Kondo Koji - Slider/Kondo Koji - Slider (NikoSek) [YaHoo!! x1.1].osu").unwrap()).unwrap();
        let bpm = calculate_bpm(&map.timing_points);
        assert_eq!(bpm, 100);
    }
}
