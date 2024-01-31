use dictionary_api::{get_definition, UnknownPartOfSpeech};

mod dictionary;
mod dictionary_api;
mod random_word_api;

pub use dictionary::{Word, PartOfSpeech, Phonetic, WordDefinition, WordMeaning};
use random_word_api::get_random_words;

#[derive(Debug)]
pub enum DictionaryError {
    Fetch(reqwest::Error),
    Deserialize(reqwest::Error),
    Conversion(UnknownPartOfSpeech),
    NotFound(NotFoundError),
}

#[derive(Debug)]
pub struct NotFoundError {
    pub message: String,
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
    pub async fn get_random_word(&self, length: Option<usize>) -> Result<Vec<String>, DictionaryError> {
        get_random_words(&self.client, 1, length).await
    }
    pub async fn get_random_words(&self, max: usize, length: Option<usize>) -> Result<Vec<String>, DictionaryError> {
        get_random_words(&self.client, max, length).await
    }

    
}