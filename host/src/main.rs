// Required for SP1
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod error;
pub mod execution;
pub mod preflight;
pub mod provider_db;
pub mod request;
pub mod server;

use std::{fmt::Debug, fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use server::serve;
use structopt::StructOpt;

#[derive(StructOpt, Default, Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Opt {
    #[structopt(long, require_equals = true, default_value = "0.0.0.0:8080")]
    /// Server bind address
    /// [default: 0.0.0.0:8080]
    address: String,

    #[structopt(long, require_equals = true, default_value = "16")]
    /// Limit the max number of in-flight requests
    concurrency_limit: usize,

    #[structopt(long, require_equals = true, default_value = "host/config/config.json")]
    /// Path to a config file that includes sufficent json args to request 
    /// a proof of specified type. Curl json-rpc overrides its contents
    config_path: PathBuf,

    #[structopt(long, require_equals = true)]
    /// Use a local directory as a cache for input. Accepts a custom directory.
    cache: Option<PathBuf>,

    #[structopt(long, require_equals = true, env = "RUST_LOG", default_value = "info")]
    /// Set the log level
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();
    let config = get_config(None).unwrap();
    println!("Start config: {:?}", config);
    
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(&opt.log_level)
        .with_test_writer()
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    serve(opt).await?;
    Ok(())
}

/// Gets the config going through all possible sources
fn get_config(request_config: Option<Value>) -> Result<Value> {
    let mut config = Value::default();
    let opt = Opt::from_args();
    println!("     cli_args: {:?}", opt);

    // Config file has the lowest preference
    let file = File::open(&opt.config_path)?;
    let reader = BufReader::new(file);
    let file_config: Value = serde_json::from_reader(reader)?;
    merge(&mut config, &file_config);
    println!("     config_path {:? }Config: {:?}", &opt.config_path, config);

    // Command line values have higher preference
    let cli_config = serde_json::to_value(&opt)?;
    merge(&mut config, &cli_config);

    // Values sent via json-rpc have the highest preference
    if let Some(request_config) = request_config {
        merge(&mut config, &request_config);
    };

    Ok(config)
}

/// Merges two json's together, overwriting `a` with the values of `b`
fn merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                merge(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b.clone(),
    }
}
