use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Generic print error")]
    PrintError,
}