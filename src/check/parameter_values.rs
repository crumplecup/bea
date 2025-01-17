use crate::{
    trace_init, BeaErr, BeaResponse, Dataset, EnvError, IoError, ParameterName, Request,
    ReqwestError,
};
use strum::IntoEnumIterator;

#[tracing::instrument]
pub async fn parameter_values_to_json() -> Result<(), BeaErr> {
    Dataset::parameter_values().await
}

#[tracing::instrument]
pub fn parameter_value_json_to_bin() -> Result<(), BeaErr> {
    trace_init();
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
            let file = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(source) => {
                    let error = IoError::new(path, source, line!(), file!().to_string());
                    return Err(error.into());
                }
            };
            let rdr = std::io::BufReader::new(file);
            // Deserialize to serde_json::Value.
            let res: serde_json::Value = serde_json::from_reader(rdr)?;
            // Serialize to binary.
            let contents = serde_json::to_vec(&res)?;
            // Set path for binary file.
            let path = format!("{bea_data}/parameter_values/{dataset}_{name}_parameter_values.bin");
            let path = std::path::PathBuf::from(path);
            // Write binary to file.
            match std::fs::write(&path, contents) {
                Ok(()) => {}
                Err(source) => {
                    let error = IoError::new(path, source, line!(), file!().to_string());
                    return Err(error.into());
                }
            }
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
pub fn parameter_value_from_json(path: std::path::PathBuf) -> Result<(), BeaErr> {
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(source) => {
            let error = IoError::new(path, source, line!(), file!().to_string());
            return Err(error.into());
        }
    };
    let rdr = std::io::BufReader::new(file);
    let res: serde_json::Value = serde_json::from_reader(rdr)?;
    let data = BeaResponse::try_from(&res)?;
    tracing::info!("Response read.");
    tracing::trace!("Response: {data:#?}");
    Ok(())
}

#[tracing::instrument(skip_all)]
pub fn parameter_value_from_bin(path: std::path::PathBuf) -> Result<(), BeaErr> {
    let decode = match std::fs::read(&path) {
        Ok(data) => data,
        Err(source) => {
            let error = IoError::new(path, source, line!(), file!().to_string());
            return Err(error.into());
        }
    };
    tracing::info!("Path read.");
    let data: serde_json::Value = serde_json::from_slice(&decode)?;
    let data = BeaResponse::try_from(&data)?;
    tracing::info!("Native: {data:#?}");
    Ok(())
}

/// reads response and native format from file
/// avoids making api calls to bea
/// used to test internal parsing of responses
#[tracing::instrument]
pub fn parameter_value_from_file() -> Result<(), BeaErr> {
    trace_init();
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
    trace_init();
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

                        let contents = serde_json::to_vec(&json)?;
                        dotenvy::dotenv().ok();
                        let bea_data = EnvError::from_env("BEA_DATA")?;
                        let path = std::path::PathBuf::from(&format!(
                            "{bea_data}/values_api_error.json" // "{bea_data}/values_{dataset}_{name}.json"
                        ));
                        match std::fs::write(&path, contents) {
                            Ok(()) => {}
                            Err(source) => {
                                let error =
                                    IoError::new(path, source, line!(), file!().to_string());
                                return Err(error.into());
                            }
                        }
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
