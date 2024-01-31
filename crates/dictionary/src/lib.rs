use dictionary_api::{get_definition, UnknownPartOfSpeech};

mod dictionary;
mod dictionary_api;

pub use dictionary::{Word, PartOfSpeech, Phonetic, WordDefinition, WordMeaning};

#[derive(Debug)]
pub enum DictionaryError {
    Fetch(reqwest::Error),
    Deserialize(reqwest::Error),
    Conversion(UnknownPartOfSpeech),
    NotFound(NotFoundError),
}

#[derive(Debug)]
pub struct NotFoundError {
    message: String,
}

pub struct Dictionary {
    client: reqwest::Client,
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_definition(&self, word: &str) -> Result<Word, DictionaryError> {
        get_definition(&self.client, word).await
    }
}