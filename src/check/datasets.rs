use crate::{
    trace_init, BeaErr, BeaResponse, Dataset, DatasetMissing, EnvError, IoError, ReqwestError,
};
use strum::IntoEnumIterator;

/// Reads response to json using serde_json.
/// Saves the result to the `data` folder.
/// Pings the BEA API.
#[tracing::instrument]
pub async fn datasets_to_json() -> Result<(), BeaErr> {
    let req = super::Request::Dataset;
    let app = req.init()?;
    let data = app.get().await?;
    match data.json::<serde_json::Value>().await {
        Ok(json) => {
            let contents = serde_json::to_vec(&json)?;
            dotenvy::dotenv().ok();
            let bea_data = EnvError::from_env("BEA_DATA")?;
            let path = std::path::PathBuf::from(&format!("{bea_data}/datasets.json"));
            IoError::write(&contents, path)?;
            Ok(())
        }
        Err(source) => {
            let url = app.url().to_string();
            let method = "get".to_string();
            let error = ReqwestError::new(url, method, source);
            Err(error.into())
        }
    }
}

/// reads response and native format from file
/// avoids making api calls to bea
/// used to test internal parsing of responses
#[tracing::instrument]
pub fn datasets_from_file() -> Result<(), BeaErr> {
    trace_init();
    dotenvy::dotenv().ok();
    let bea_data = EnvError::from_env("BEA_DATA")?;
    tracing::info!("BEA_DATA present.");
    let path = std::path::PathBuf::from(&format!("{bea_data}/datasets.json"));
    let file = IoError::open(path)?;
    tracing::info!("File opened.");
    let rdr = std::io::BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(rdr)?;
    let bea = BeaResponse::try_from(&json)?;
    tracing::info!("Response: {bea:#?}");

    let path = std::path::PathBuf::from(&format!("{bea_data}/datasets.bin"));
    tracing::info!("Reading {path:?}.");
    let decode = IoError::read(path)?;
    tracing::info!("File read to Vec<u8>.");
    let data: serde_json::Value = serde_json::from_slice(&decode)?;
    let data = BeaResponse::try_from(&data)?;
    tracing::info!("Native: {data:#?}");
    Ok(())
}

/// Deserialize directly from response.
/// Write to binary format in the BEA_DATA directory.
/// Pings the BEA API.
#[tracing::instrument]
pub async fn deserialize_datasets() -> Result<(), BeaErr> {
    let req = super::Request::Dataset;
    let app = req.init()?;
    let data = app.get().await?;
    match data.json::<BeaResponse>().await {
        Ok(datasets) => {
            tracing::info!("Result: {:#?}", datasets);
            let encode = BeaResponse::serialize(&datasets)?;
            dotenvy::dotenv().ok();
            let bea_data = EnvError::from_env("BEA_DATA")?;
            let path = std::path::PathBuf::from(&format!("{bea_data}/datasets.bin"));
            tracing::info!("Writing to path {:?}", path);
            IoError::write(&encode, path)?;
            Ok(())
        }
        Err(source) => {
            let url = app.url().to_string();
            let method = "get".to_string();
            let body = app.params().into_iter().collect::<Vec<(String, String)>>();
            let mut error = ReqwestError::new(url, method, source);
            if !body.is_empty() {
                error.with_body(body);
            }
            Err(error.into())
        }
    }
}

/// Checks that each dataset returned from the call matches an enum variant in Dataset
/// Returns an error if the datasets do not match.
#[tracing::instrument]
pub fn check_datasets() -> Result<(), BeaErr> {
    trace_init();
    dotenvy::dotenv().ok();
    let bea_data = EnvError::from_env("BEA_DATA")?;
    let path = std::path::PathBuf::from(&format!("{bea_data}/datasets.json"));
    let file = IoError::open(path)?;
    let rdr = std::io::BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(rdr)?;
    let data = BeaResponse::try_from(&json)?;
    tracing::trace!("Response: {data:#?}");
    let sets: Vec<String> = Dataset::iter().map(|d| d.lower()).collect();
    tracing::info!("Sets: {:#?}", sets);

    if let Some(datasets) = data.datasets() {
        for dataset in datasets.iter() {
            let name = dataset.dataset_name().to_lowercase();
            if !sets.contains(&name) {
                tracing::warn!("{} not in datasets.", dataset.dataset_name());
                let error = DatasetMissing::from(dataset.dataset_name());
                return Err(error.into());
            } else {
                tracing::info!("{} in datasets.", dataset.dataset_name());
            }
        }
    }
    Ok(())
}

#[tracing::instrument]
pub fn datasets_json_to_bin() -> Result<(), BeaErr> {
    trace_init();
    dotenvy::dotenv().ok();
    let bea_data = EnvError::from_env("BEA_DATA")?;
    // Set path for json file.
    let path = format!("{bea_data}/datasets.json");
    let path = std::path::PathBuf::from(path);
    // Create reader from path.
    let file = IoError::open(path)?;
    let rdr = std::io::BufReader::new(file);
    // Deserialize to serde_json::Value.
    let res: serde_json::Value = serde_json::from_reader(rdr)?;
    // Serialize to binary.
    let encode = serde_json::to_vec(&res)?;
    // Set path for binary file.
    let path = format!("{bea_data}/datasets.bin");
    let path = std::path::PathBuf::from(path);
    // Write binary to file.
    IoError::write(&encode, path)?;
    Ok(())
}
