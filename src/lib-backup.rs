use std::{path::PathBuf, io::{BufReader, BufWriter}, fs::File, io::Write, io::Read, borrow::BorrowMut};
use anyhow::Result;
use osuparse::{HitCircle, HitObject};
use libosu::{prelude::*, errors::ParseError};
pub async fn generate_map(path: &PathBuf, rate: f64) -> Result<()>{
    let map_file = File::open(path)?;
    let contents = String::new();
    map_file.read_to_string(&mut contents)?;
    let mut map_data = osuparse::parse_beatmap(contents.as_str()).unwrap();
    for h in &mut map_data.hit_objects{
        match &mut h{
            HitObject::HitCircle(c) => c.time = (rate * c.time as f64).round() as i32,
            HitObject::HoldNote(s) =>{
                s.time = (rate * s.time as f64).round() as i32;
                s.end_time = (rate * s.end_time as f64).round() as i32;
            },
            HitObject::Slider(c) => c.time = (rate * c.time as f64).round() as i32,
            HitObject::Spinner(c) => c.time = (rate * c.time as f64).round() as i32,
        }
    }
    for t in &mut map_data.timing_points{
        t.offset = (rate * t.offset as f64).round() as f32;
    }
    // for h in &mut map_data.hit_objects{
    //     h.start_time.0 = (rate * *h.start_time as f64).round() as i32;
    // }
    // for t in &mut map_data.timing_points{
    //     t.time.0 = (rate * t.time.0 as f64).round() as i32;
    // }
    write!(File::create(format!("{}({})", path.to_str().unwrap(), rate))?,"{}", map_data)?;
    Ok(())
}

#[cfg(test)]
mod test{
    use super::*;
    #[tokio::test]
    async fn test(){
        generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/983584 YMCK - Curry Da Yo!/YMCK - Curry Da Yo! (qqqant) [Ricetune-].osu"), 1.3).await.unwrap();
    }
}
