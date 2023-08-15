use std::{path::PathBuf, io::{BufReader, BufWriter}, fs::File, io::Write, borrow::BorrowMut};
use anyhow::Result;
pub async fn generate_map(path: &PathBuf, rate: f64) -> Result<()>{
    let map_file = File::open(path)?;
    let mut map_data = libosu::beatmap::Beatmap::parse(map_file)?;
    for h in &mut map_data.hit_objects{
        h.start_time.0 = (rate * *h.start_time as f64).round() as i32;
    }
    for t in &mut map_data.timing_points{
        t.time.0 = (rate * t.time.0 as f64).round() as i32;
    }
    let new_path = path.parent().unwrap().join(path.file_stem().unwrap());
    write!(File::create(format!("{}({}).osu", new_path.display(), rate))?,"{}", map_data)?;
    Ok(())
}

#[cfg(test)]
mod test{
    use super::*;
    #[tokio::test]
    async fn test(){
        generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/991895 Kondo Koji - Slider/Kondo Koji - Slider (NikoSek) [YaHoo!!].osu"), 1.5).await.unwrap();
    }
}
