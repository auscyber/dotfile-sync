use snafu::Snafu;

use crate::config::ProjectConfig;
#[derive(Snafu, Debug)]

pub enum Error {
    #[snafu(display("No files found to link"))]
    NoFiles,
    #[snafu(display("Failed to manage project `{}`: {} ",config.name,source))]
    #[snafu(visibility(pub(crate)))]
    Management {
        config: ProjectConfig,
        source: crate::actions::ManagementError,
    },
}
