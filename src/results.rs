use crate::{
    error::ParseInt, map_to_string, BeaErr, BincodeError, Data, Datasets, JsonParseError,
    JsonParseErrorKind, KeyMissing, Method, NipaData, NotObject, ParameterValues, Parameters,
    RequestParameters,
};

#[derive(
    Clone, Debug, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize, derive_more::From,
)]
pub enum Results {
    #[from(ApiError)]
    ApiError(ApiError),
    #[from(Data)]
    Data(Data),
    #[from(Datasets)]
    Datasets(Datasets),
    #[from(Parameters)]
    Parameters(Parameters),
    #[from(ParameterValues)]
    ParameterValues(ParameterValues),
}

impl Results {
    #[tracing::instrument(skip_all)]
    pub fn into_datasets(&self) -> Option<Datasets> {
        match self {
            Self::Datasets(d) => Some(d.clone()),
            _ => None,
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn into_parameters(&self) -> Option<Parameters> {
        match self {
            Self::Parameters(p) => Some(p.clone()),
            _ => None,
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn into_parameter_values(&self) -> Option<ParameterValues> {
        match self {
            Self::ParameterValues(p) => Some(p.clone()),
            _ => None,
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn read_json(
        value: &serde_json::Value,
        request: &RequestParameters,
    ) -> Result<Self, BeaErr> {
        let method = request.method()?;
        match method {
            Method::GetData => {
                tracing::info!("Trying Data...");
                if let Ok(t) = NipaData::try_from(value) {
                    let data = Data::from(t);
                    return Ok(Self::from(data));
                }
            }
            Method::GetDataSetList => {
                tracing::trace!("Trying datasets...");
                if let Ok(t) = Datasets::try_from(value.clone()) {
                    return Ok(Self::from(t));
                }
            }
            Method::GetParameterList => {
                tracing::trace!("Trying parameters...");
                match Parameters::try_from(value) {
                    Ok(t) => {
                        tracing::trace!("Parameters found, returning...");
                        return Ok(Self::from(t));
                    }
                    Err(source) => {
                        tracing::trace!("{source}");
                    }
                }
            }
            Method::GetParameterValues | Method::GetParameterValuesFiltered => {
                tracing::trace!("Trying parameter values...");
                if let Ok(t) = ParameterValues::try_from(value) {
                    return Ok(Self::from(t));
                }
            }
        }
        tracing::trace!("Trying ApiError...");
        if let Ok(t) = ApiError::try_from(value) {
            return Ok(Self::from(t));
        }
        let error = KeyMissing::new(
            "Not results table.".to_string(),
            line!(),
            file!().to_string(),
        );
        let error = JsonParseErrorKind::from(error);
        let error = JsonParseError::from(error);
        Err(error.into())
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    serde::Deserialize,
    serde::Serialize,
    derive_new::new,
    derive_getters::Getters,
)]
#[serde(rename_all = "PascalCase")]
pub struct Beaapi {
    request: RequestParameters,
    results: Results,
}

impl Beaapi {
    #[tracing::instrument(skip_all)]
    pub fn read_json(m: &serde_json::Map<String, serde_json::Value>) -> Result<Self, BeaErr> {
        let key = "Request".to_string();
        let request = if let Some(value) = m.get(&key) {
            RequestParameters::try_from(value)?
        } else {
            let error = KeyMissing::new(key, line!(), file!().to_string());
            let error = JsonParseErrorKind::from(error);
            let error = JsonParseError::from(error);
            return Err(error.into());
        };
        let key = "Results".to_string();
        let results = if let Some(value) = m.get(&key) {
            Results::read_json(value, &request)?
        } else {
            let error = KeyMissing::new(key, line!(), file!().to_string());
            let error = JsonParseErrorKind::from(error);
            let error = JsonParseError::from(error);
            return Err(error.into());
        };
        let beaapi = Self::new(request, results);
        Ok(beaapi)
    }
}

impl TryFrom<&serde_json::Value> for Beaapi {
    type Error = BeaErr;
    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        tracing::trace!("Reading Beaapi.");
        match value {
            serde_json::Value::Object(m) => {
                let bea = Self::read_json(m)?;
                Ok(bea)
            }
            _ => {
                tracing::warn!("Invalid Value: {value:#?}");
                let error = NotObject::new(line!(), file!().to_string());
                let error = JsonParseErrorKind::from(error);
                let error = JsonParseError::from(error);
                Err(error.into())
            }
        }
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    serde::Deserialize,
    serde::Serialize,
    derive_more::From,
    derive_more::Deref,
    derive_more::DerefMut,
)]
#[from(Beaapi)]
pub struct BeaResponse {
    #[serde(rename = "BEAAPI")]
    beaapi: Beaapi,
}

impl BeaResponse {
    #[tracing::instrument(skip_all)]
    pub fn datasets(&self) -> Option<Datasets> {
        self.beaapi.results.into_datasets()
    }

    #[tracing::instrument(skip_all)]
    pub fn parameters(&self) -> Option<Parameters> {
        self.beaapi.results.into_parameters()
    }

    #[tracing::instrument(skip_all)]
    pub fn parameter_values(&self) -> Option<ParameterValues> {
        self.beaapi.results.into_parameter_values()
    }

    #[tracing::instrument(skip_all)]
    pub fn serialize(&self) -> Result<Vec<u8>, BincodeError> {
        match bincode::serialize(self) {
            Ok(data) => Ok(data),
            Err(source) => {
                let error = BincodeError::new("serializing BeaResponse".to_string(), source);
                Err(error)
            }
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn deserialize(bytes: &[u8]) -> Result<Self, BincodeError> {
        tracing::trace!("Deserializing BeaResponse");
        match bincode::deserialize(bytes) {
            Ok(data) => Ok(data),
            Err(source) => {
                let error = BincodeError::new("deserializing BeaResponse".to_string(), source);
                Err(error)
            }
        }
    }

    pub fn into_parts(&self) -> (RequestParameters, Results) {
        let req = self.request().clone();
        let res = self.results().clone();
        (req, res)
    }
}

impl TryFrom<&serde_json::Value> for BeaResponse {
    type Error = BeaErr;
    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        tracing::trace!("Reading BeaResponse.");
        match value {
            serde_json::Value::Object(m) => {
                let key = "BEAAPI".to_string();
                if let Some(val) = m.get(&key) {
                    tracing::trace!("Val is: {val:#?}");
                    let beaapi = Beaapi::try_from(val)?;
                    Ok(Self { beaapi })
                } else {
                    tracing::trace!("Invalid Object: {m:#?}");
                    let error = KeyMissing::new(key, line!(), file!().to_string());
                    let error = JsonParseErrorKind::from(error);
                    let error = JsonParseError::from(error);
                    Err(error.into())
                }
            }
            _ => {
                tracing::trace!("Invalid Value: {value:#?}");
                let error = NotObject::new(line!(), file!().to_string());
                let error = JsonParseErrorKind::from(error);
                let error = JsonParseError::from(error);
                Err(error.into())
            }
        }
    }
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Deserialize,
    serde::Serialize,
    derive_new::new,
    derive_more::Display,
)]
#[display("API Error - Code: {} - Description: {}", self.code, self.description)]
pub struct ApiError {
    #[serde(rename = "APIErrorCode")]
    code: i32,
    #[serde(rename = "APIErrorDescription")]
    description: String,
}

impl ApiError {
    pub fn read_json(mp: &serde_json::Map<String, serde_json::Value>) -> Result<Self, BeaErr> {
        let key = "Error".to_string();
        if let Some(value) = mp.get(&key) {
            match value {
                serde_json::Value::Object(m) => {
                    let description = map_to_string("APIErrorDescription", m)?;
                    let code = map_to_string("APIErrorCode", m)?;
                    match str::parse::<i32>(&code) {
                        Ok(code) => Ok(Self::new(code, description)),
                        Err(source) => {
                            let line = line!();
                            let file = file!();
                            let error = ParseInt::new(code, source, line, file.into());
                            Err(error.into())
                        }
                    }
                }
                _ => {
                    tracing::trace!("Invalid Value: {value:#?}");
                    let error = NotObject::new(line!(), file!().to_string());
                    let error = JsonParseErrorKind::from(error);
                    let error = JsonParseError::from(error);
                    Err(error.into())
                }
            }
        } else {
            let error = KeyMissing::new(key, line!(), file!().to_string());
            let error = JsonParseErrorKind::from(error);
            let error = JsonParseError::from(error);
            Err(error.into())
        }
    }
}

impl TryFrom<&serde_json::Value> for ApiError {
    type Error = BeaErr;
    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        tracing::trace!("Reading ApiError.");
        match value {
            serde_json::Value::Object(m) => ApiError::read_json(m),
            _ => {
                tracing::trace!("Invalid Value: {value:#?}");
                let error = NotObject::new(line!(), file!().to_string());
                let error = JsonParseErrorKind::from(error);
                let error = JsonParseError::from(error);
                Err(error.into())
            }
        }
    }
}
