use crate::error::{Error, Result};
use std::env;

#[derive(PartialEq, Debug)]
pub(crate) enum Update {
    Wip,
    Overwrite,
}

impl Default for Update {
    fn default() -> Self {
        Update::Wip
    }
}

impl Update {
    pub fn env() -> Result<Self> {
        let var = match env::var_os("TRYBUILD") {
            Some(var) => var,
            None => {
                if env::var_os("CI").is_none() {
                    return Ok(Update::Overwrite);
                } else {
                    return Ok(Update::default());
                }
            }
        };

        match var.as_os_str().to_str() {
            Some("wip") => Ok(Update::Wip),
            Some("overwrite") => Ok(Update::Overwrite),
            _ => Err(Error::UpdateVar(var)),
        }
    }
}
