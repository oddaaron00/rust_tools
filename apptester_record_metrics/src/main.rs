#![feature(fs_try_exists)]

use dotenv::dotenv;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    collections::HashMap,
    env::args,
    error::Error,
    fs::{self, File},
    io::{self, BufRead, LineWriter, Read, Write},
    path::Path,
};

fn main() {
    dotenv().ok();
    if let Err(err) = run() {
        eprintln!("ERROR: {err}");
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let output_location = match args().nth(1) {
        Some(val) => val,
        None => return Err("Argument for output file path required".into()),
    };
    if let Ok(true) = fs::try_exists(&output_location) {
        return Err("File already exists".into());
    }

    let client = reqwest::blocking::Client::new();
    let mut buf = String::new();

    let client_url = std::env::var("CLIENT_URL")?;
    client
        .get(format!("{client_url}/sessions"))
        .send()?
        .read_to_string(&mut buf)?;
    let res: Value = serde_json::from_str(&buf)?;
    let res = res.get("value").unwrap().as_array().unwrap();
    if res.is_empty() {
        return Err("Session not started".into());
    }

    let res = res.get(0).unwrap().as_object().unwrap();
    let session_id = res.get("id").unwrap().as_str().unwrap();

    let output_file = File::create(Path::new(&output_location))?;
    let mut output_file = LineWriter::new(output_file);
    let handle = io::stdin().lock();

    output_file.write_all(b"feature,stage,dalvikPrivateDirty,dalvikPss,dalvikRss,eglPrivateDirty,eglPss,glPrivateDirty,glPss,nativeHeapAllocatedSize,nativeHeapSize,nativePrivateDirty,nativePss,nativeRss,totalPrivateDirty,totalPss,totalRss\n")?;

    for line in handle.lines() {
        let line = line?;
        if let Err(err) = parse_input(&client, session_id, line, &mut output_file) {
            eprintln!("{err}");
        }
    }
    output_file.flush()?;

    Ok(())
}

fn parse_input(
    client: &Client,
    session_id: &str,
    line: String,
    output_file: &mut LineWriter<File>,
) -> Result<(), Box<dyn Error>> {
    let mut buf = String::new();
    let (feature, stage) = match line.split_once(' ') {
        Some((feature, stage)) => (feature, stage),
        _ => return Err("Cannot split input into feature and stage".into()),
    };
    if stage
        .bytes()
        .fold(0, |acc, e| if e == b' ' { acc + 1 } else { acc })
        != 0
    {
        return Err("Input must be of the format FEATURE STAGE".into());
    }
    if stage != "start" && stage != "stop" {
        return Err("Stage must be either 'start' or 'stop'".into());
    }

    let client_url = std::env::var("CLIENT_URL")?;
    let package_name = std::env::var("PACKAGE_NAME")?;
    client
        .post(format!(
            "{client_url}/session/{session_id}/appium/getPerformanceData"
        ))
        .body(format!(
            "{{\"packageName\":\"{package_name}\",\"dataType\":\"memoryinfo\"}}"
        ))
        .send()?
        .read_to_string(&mut buf)?;
    let res: Value = serde_json::from_str(&buf)?;
    let res = res.get("value").unwrap();

    if let Some(res) = res.get("error") {
        let err = res.get("error").unwrap().as_str().unwrap();
        return Err(err.into());
    }

    output_file.write_all(format!("{feature},{stage},").as_bytes())?;

    if let Some(arrays) = res.as_array() {
        let vals = vals_from_arrays(arrays);
        write_vals_to_file(output_file, vals)?;
    }

    output_file.write_all(b"\n")?;
    Ok(())
}

fn write_vals_to_file(
    output_file: &mut LineWriter<File>,
    vals: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    output_file.write_all(vals.join(",").as_bytes())?;
    Ok(())
}

fn vals_from_sorted_val_vec(val_vec: Vec<(String, String)>) -> Vec<String> {
    val_vec
        .iter()
        .map(|(_, s)| s.to_owned())
        .collect::<Vec<String>>()
}

fn sorted_val_vec_from_val_map(val_map: HashMap<String, String>) -> Vec<(String, String)> {
    let mut val_vec: Vec<(String, String)> = val_map.into_iter().collect();
    val_vec.sort_by_key(|k| k.clone().0);
    val_vec
}

fn vals_from_arrays(arrays: &[Value]) -> Vec<String> {
    let (&arr0, &arr1) = (
        &arrays[0].as_array().unwrap(),
        &arrays[1].as_array().unwrap(),
    );
    let mut val_map: HashMap<String, String> = HashMap::new();

    for (val0, val1) in arr0.iter().zip(arr1.iter()) {
        val_map.insert(
            val0.as_str().unwrap().to_string(),
            val1.as_str().unwrap_or("").to_string(),
        );
    }

    let val_vec = sorted_val_vec_from_val_map(val_map);

    vals_from_sorted_val_vec(val_vec)
}
