use crate::{
    trace_init, BeaErr, BeaResponse, Dataset, EnvError, IoError, ParameterName, Request,
    ReqwestError, SerdeJson,
};
use strum::IntoEnumIterator;

#[tracing::instrument]
pub async fn parameter_values_to_json() -> Result<(), BeaErr> {
    Dataset::parameter_values().await
}

#[tracing::instrument]
pub fn parameter_value_json_to_bin() -> Result<(), BeaErr> {
    trace_init()?;
    dotenvy::dotenv().ok();
    let datasets: Vec<Dataset> = Dataset::iter().collect();
    let bea_data = EnvError::from_env("BEA_DATA")?;
    for dataset in datasets {
        let names = dataset.names();
        for name in names {
            // Set path for json file.
            let path =
                format!("{bea_data}/parameter_values/{dataset}_{name}_parameter_values.json");
            let path = std::path::PathBuf::from(path);
            // Create reader from path.
            let file = std::fs::File::open(&path)
                .map_err(|e| IoError::new(path, e, line!(), file!().into()))?;
            let rdr = std::io::BufReader::new(file);
            // Deserialize to serde_json::Value.
            let res: serde_json::Value = serde_json::from_reader(rdr)
                .map_err(|e| SerdeJson::new(e, line!(), file!().to_string()))?;
            // Serialize to binary.
            let contents = serde_json::to_vec(&res)
                .map_err(|e| SerdeJson::new(e, line!(), file!().to_string()))?;
            // Set path for binary file.
            let path = format!("{bea_data}/parameter_values/{dataset}_{name}_parameter_values.bin");
            let path = std::path::PathBuf::from(path);
            // Write binary to file.
            std::fs::write(&path, contents)
                .map_err(|e| IoError::new(path, e, line!(), file!().into()))?;
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
pub fn parameter_value_from_json(path: std::path::PathBuf) -> Result<(), BeaErr> {
    let file =
        std::fs::File::open(&path).map_err(|e| IoError::new(path, e, line!(), file!().into()))?;
    let rdr = std::io::BufReader::new(file);
    let res: serde_json::Value = serde_json::from_reader(rdr)
        .map_err(|e| SerdeJson::new(e, line!(), file!().to_string()))?;
    let data = BeaResponse::try_from(&res)?;
    tracing::info!("Response read.");
    tracing::trace!("Response: {data:#?}");
    Ok(())
}

#[tracing::instrument(skip_all)]
pub fn parameter_value_from_bin(path: std::path::PathBuf) -> Result<(), BeaErr> {
    let decode =
        std::fs::read(&path).map_err(|e| IoError::new(path, e, line!(), file!().into()))?;
    tracing::info!("Path read.");
    let data: serde_json::Value = serde_json::from_slice(&decode)
        .map_err(|e| SerdeJson::new(e, line!(), file!().to_string()))?;
    let data = BeaResponse::try_from(&data)?;
    tracing::info!("Native: {data:#?}");
    Ok(())
}

/// reads response and native format from file
/// avoids making api calls to bea
/// used to test internal parsing of responses
#[tracing::instrument]
pub fn parameter_value_from_file() -> Result<(), BeaErr> {
    trace_init()?;
    dotenvy::dotenv().ok();
    let datasets: Vec<Dataset> = Dataset::iter().collect();
    let bea_data = EnvError::from_env("BEA_DATA")?;
    for dataset in datasets {
        let names = dataset.names();
        for name in names {
            tracing::info!("Response pass for {name} in {dataset}");
            let path = std::path::PathBuf::from(&format!(
                "{bea_data}/parameter_values/{dataset}_{name}_parameter_values.json"
            ));
            parameter_value_from_json(path)?;

            tracing::info!("Native pass for {name} in {dataset}");
            let path = std::path::PathBuf::from(&format!(
                "{bea_data}/parameter_values/{dataset}_{name}_parameter_values.bin"
            ));
            tracing::info!("Reading from {path:?}");
            parameter_value_from_bin(path)?;
        }
    }
    Ok(())
}

#[tracing::instrument]
pub async fn parameter_value_filtered() -> Result<(), BeaErr> {
    trace_init()?;
    let req = Request::ParameterValueFilter;
    let mut app = req.init()?;
    let datasets: Vec<Dataset> = Dataset::iter().collect();
    for dataset in &datasets {
        let names = dataset.names();
        for name in names {
            if *dataset == Dataset::Nipa && name == ParameterName::Frequency {
                let mut options = app.options().clone();
                options.with_dataset(*dataset);
                options.with_target(name);
                app.add_options(options.clone());
                let data = app.get().await?;
                tracing::info!("{data:#?}");
                match data.json::<serde_json::Value>().await {
                    Ok(json) => {
                        tracing::info!("{json:#?}");
                        // let bea = BeaResponse::try_from(&json)?;
                        // match bea.results() {
                        //     Results::ParameterValues(pv) => {}
                        //     Results::ApiError(e) => {
                        //         tracing::info!("{e}");
                        //     }
                        //     unexpected => {
                        //         tracing::warn!("Unexpected type {unexpected:#?}");
                        //     }
                        // }

                        let contents = serde_json::to_vec(&json)
                            .map_err(|e| SerdeJson::new(e, line!(), file!().to_string()))?;
                        dotenvy::dotenv().ok();
                        let bea_data = EnvError::from_env("BEA_DATA")?;
                        let path = std::path::PathBuf::from(&format!(
                            "{bea_data}/values_api_error.json" // "{bea_data}/values_{dataset}_{name}.json"
                        ));
                        std::fs::write(&path, contents)
                            .map_err(|e| IoError::new(path, e, line!(), file!().into()))?;
                    }
                    Err(source) => {
                        let url = app.url().to_string();
                        let method = "get".to_string();
                        let body = app.params().into_iter().collect::<Vec<(String, String)>>();
                        let mut error =
                            ReqwestError::new(url, method, source, line!(), file!().to_string());
                        error.with_body(body);
                        return Err(error.into());
                    }
                }
            }
        }
    }
    Ok(())
}
