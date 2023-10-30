#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::result_large_err)]

use snafu::prelude::*;

#[cfg(feature = "tracing")]
use tracing_subscriber::prelude::*;

#[derive(Debug, Snafu)]
pub enum Error {
    TracingSubscriberTryInit {
        source: tracing_subscriber::util::TryInitError,
    },
}

#[tokio::main]
async fn main() -> Result<(), self::Error> {
    #[cfg(feature = "tracing")]
    tracing_subscriber::registry()
        .with(tracing_forest::ForestLayer::default())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .context(TracingSubscriberTryInitSnafu)?;

    Ok(())
}
